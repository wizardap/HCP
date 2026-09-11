# Technical Design Specification: AlternatingPortEngine (Hybrid 2-Tier Alternating Cycle Engine for CEGAR-Fix)

- **Date:** 2026-09-11
- **Status:** Approved
- **Target File:** `docs/superpowers/specs/2026-09-11-alternating-port-engine-design.md`
- **Target Component:** `src/cegar-fix` (Rust solver core)

---

## 1. Executive Summary & Problem Motivation

### 1.1 The Phenomenon on Class B2a (`graph868.col`)
On Class B2a hard instances (e.g. `graph868.col`, $N=5,544 \to 3,696$), the Degree-2 Contracted graph possesses a strict bipartite block structure and contains symmetric gadget clusters. 

In this structure:
1. Every valid 2-factor is a collection of disjoint cycles with even block lengths.
2. Standard 2-opt and local edge swaps cannot merge cycles without breaking bipartite block parity (`number of connected cycles = 0` in 100% of CEGAR iterations).
3. The native SAT-CEGAR loop stalls at $\sim 42-47$ subcycles because cutting dormant cycles of length $\ge 8$ fails to trigger unit propagation in CaDiCaL, causing exponential conflict escalation ($240,144$ conflicts at Iter 7).
4. Conversely, offline experimental prototypes (`test_alternating_engine.rs`) demonstrated that alternating cycle flips (exchanging active port connections with inactive port connections) can reduce subcycles from $31 \to 8 \to 3$ in under $110$ms with zero invalid edges.

### 1.2 Proposed Solution
Implement `AlternatingPortEngine` as a core module in `src/cegar-fix/src/alternating_port_engine.rs`, wired directly into the main CEGAR loop in `src/cegar-fix/src/hcp_solver.rs`. When CaDiCaL returns a 2-factor solution, `AlternatingPortEngine` is immediately invoked to perform:
- **Tier 1**: 1-step greedy alternating cycle flips ($<100$ms) to compress the cycle count to a minimal set.
- **Tier 2**: Bounded multi-hop BFS escape search ($\le 1.0$s) targeting satellite subcycles to connect them into the dominant giant cycle.
- **Immediate Tour Detection**: If cycle count reaches 1, the tour is verified, uncontracted, and returned as an immediate SATISFIABLE solution.
- **Reduced Cut Set**: If cycle count remains $>1$, the compressed cycle set (e.g. 3 cycles) is passed down to `CutSelector`, dramatically reducing the number and size of cuts CaDiCaL must learn.

---

## 2. Architecture & Data Structures

### 2.1 Port Abstraction
Given a contracted graph $G_{contracted}$ produced by `Degree2Contractor`:
- Each contracted degree-2 chain $(u, w)$ represents a block $b \in [0, N_{blocks})$.
- Block $b$ has two ports:
  - Input Port: $p_u = \text{Port} \{ \text{block}: b, \text{end}: 0 \}$
  - Output Port: $p_w = \text{Port} \{ \text{block}: b, \text{end}: 1 \}$
- Invariant: A Hamiltonian path exists internally between $(b, 0)$ and $(b, 1)$ traversing the contracted chain vertices.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Port {
    pub block: usize,
    pub end: u8,
}
```

### 2.2 Bidirectional Mapping
`AlternatingPortEngine` maintains:
1. `node_to_port: HashMap<i32, Port>`: Maps each vertex in $G_{contracted}$ to its corresponding block port.
2. `port_to_node: HashMap<Port, i32>`: Maps each `Port` back to the concrete vertex in $G_{contracted}$.
3. `active_partner: HashMap<Port, Port>`: In the current 2-factor solution, port $p$ is connected to $q$ via an active external edge $(u, v) \in E(G_{contracted})$.
4. `inactive_adj: HashMap<Port, Vec<Port>>`: All ports $q$ such that $(port\_to\_node[p], port\_to\_node[q]) \in E(G_{contracted})$ and $q \ne active\_partner[p]$.

---

## 3. Two-Tier Alternating Search Algorithm

### 3.1 Cycle Extraction from Matching
Given external edges $E_{ext}$, the cycle decomposition is extracted by following:
$$p \xrightarrow{\text{external}} active\_partner[p] \xrightarrow{\text{internal}} (active\_partner[p].block, 1 - active\_partner[p].end) \dots$$
Each cycle is represented as an ordered sequence of ports.

### 3.2 Tier 1: 1-Step Greedy Alternating Flips
1. **Auxiliary Graph Formation**:
   - For each port $p$, select an inactive partner $inact \in inactive\_adj[p]$.
   - Follow $act = active\_partner[inact]$.
   - This defines a directed auxiliary transition: $p \to inact \to act$.
2. **Cycle Finding**:
   - Trace cycles in the auxiliary transition graph. Each cycle $C_{alt}$ alternates between active and inactive edges.
3. **Evaluation & Flip**:
   - Compute candidate edge set:
     $$E' = (E_{current} \setminus \{ (p, active\_partner[p]) \mid p \in C_{alt} \}) \cup \{ (p_i, p_{i+1}) \mid \text{inactive in } C_{alt} \}$$
   - If the new cycle partition reduces the number of cycles or increases the giant cycle size without increasing total cycles, apply the flip.
   - Repeat until no further 1-step reduction is possible (fixed point reached).

### 3.3 Tier 2: Bounded Multi-Hop BFS Escape
When Tier 1 reaches a fixed point and `cycles.len() > 1`:
1. Identify small satellite cycles ($|C| \le 16$ vertices / 8 blocks) and the dominant giant cycle.
2. For each port $p_0$ in a satellite cycle, perform a bounded alternating BFS:
   - Alternates between inactive edges and active partner edges.
   - Search depth is bounded: $2 \le \text{depth} \le 4$ (4 to 8 edges).
   - Time budget guard: Stop search if elapsed time exceeds `timeout_ms` (default 1,000ms).
3. If an alternating cycle closing back to $p_0$ or bridging disjoint components is discovered:
   - Apply the multi-hop flip.
   - Return to Tier 1 greedy loop.
4. If no multi-hop flip succeeds within the budget, return the current reduced cycle set.

---

## 4. Integration into Solver Loop

### 4.1 CLI & Options
In `src/cegar-fix/src/options.rs` and `hybrid_orchestrator.rs`:
- `--alternating-engine`: Boolean flag (default `true` when `contractor.chain_map.len() > 0`).
- `--no-alternating-engine`: Disables the engine if user requests pure SAT-CEGAR.

### 4.2 Hook in `hcp_solver.rs`
Immediately after SAT extracts `sol_cycles`:
```rust
let sol_cycles = if options.alternating_engine && !contractor.chain_map.is_empty() && sol_cycles.len() > 1 {
    let repaired = AlternatingPortEngine::repair(&sol_cycles, &g, &contractor, 4, 1000);
    if repaired.len() == 1 && repaired[0].len() == g.adjacency_list.len() {
        let flat: Vec<i32> = repaired.into_iter().flatten().collect();
        let final_tour = contractor.uncontract_cycle(&flat);
        println!("*** 100% HAMILTONIAN TOUR FOUND VIA ALTERNATING PORT ENGINE! ***");
        return (count, clause_count, Some(final_tour));
    }
    repaired
} else {
    sol_cycles
};
```

---

## 5. Verification & Testing Strategy

### 5.1 Unit Tests (`tests/test_alternating_port_engine.rs`)
1. `test_synthetic_bipartite_alternating_merge`: Minimal bipartite graph with 2 disjoint 4-cycles merges into 1 Hamiltonian cycle.
2. `test_port_alternating_degree_invariants`: Verifies all port degrees are exactly 1 and all original graph degrees are exactly 2 before and after flips.
3. `test_graph868_snapshot_reduction`: Loads `scratch/graph868_8_cycles.txt`, asserts reduction to $\le 3$ cycles in $<50$ms with 0 invalid edges.

### 5.2 Integration Tests (`tests/test_alternating_integration.rs`)
1. End-to-end solve with `--alternating-engine` on contracted graphs.
2. Verification of uncontracted tour with `contractor.uncontract_cycle`.

### 5.3 Scientific Constraints
- **Zero Tour Injection**: Absolutely no `.tou` files read anywhere in the code.
- **Independent Verification**: Every discovered tour verified 100% on raw graph $G$ using `scratch/verify_benchmarks.py` logic.
- **Non-Regression**: All 50 existing test suites in `src/cegar-fix` pass cleanly.
