# Spec: Connected Port-Subpath LNS Corridor for Subcycle Absorption

**Date:** 2026-09-12  
**Status:** Approved  
**Target Graph:** `FHCPCS-col/graph868.col` and similar bipartite-parity gadget graphs  
**Implementation Directory:** `src/cegar-fix/`  

---

## 1. Goal & Motivation

In previous benchmarks on `graph868.col`, the `AlternatingPortEngine` successfully compressed subcycles from 97 down to 33, expanding the Giant Cycle to 3,256 / 3,696 vertices (88.1% of the contracted graph). However, the remaining 32 satellite cycles ($k \in \{4, 8, 16\}$ ports) could not be reduced further by pairwise alternating BFS due to long multi-hop distances ($> 12$ hops) and parity barriers.

Simultaneously, the existing `LocalizedSatRepair` failed with `UNSAT` at Hop 1 in 0.079s because it attempted to unfreeze all 33 subcycles at once, creating an unconstrained 936-vertex boundary cutting across bipartite gadget parity interfaces.

**The Solution:** Implement `PortCorridorLns`, a focused Single-Cycle Large Neighborhood Search (LNS) engine that:
1. Targets **one** small satellite cycle $C_i$ at a time (Smallest-Cycle-First).
2. Identifies its docking ports on the Giant Cycle $C_{giant}$.
3. Unfreezes only $C_i$ and the contiguous subpath of $C_{giant}$ covering the docking ports (15–30 blocks, 30–60 vertices).
4. Restricts the boundary to **exactly 2 cut edges** ($P_{entry}$ and $P_{exit}$), mathematically guaranteeing bipartite balance and eliminating the Parity Barrier.
5. Solves the localized routing in CaDiCaL in $< 5$ms per cycle, sequentially absorbing all satellite cycles into the Giant Cycle.

---

## 2. Mathematical Formulation & Invariants

### 2.1 Port-Coordinate Mapping of the Giant Cycle
In the degree-2 contracted graph, each block $b \in \{0, \dots, N_{blocks}-1\}$ consists of two ports:
$$\text{Port}(b, 0), \quad \text{Port}(b, 1)$$
The Giant Cycle $C_{giant}$ forms a circular sequence of $2L$ ports ($L = |C_{giant}| / 2$ blocks):
$$P_0, P_1, P_2, \dots, P_{2L-1}$$
where:
- Internal block transitions connect $P_{2k}$ to $P_{2k+1} = \text{Port}(b_k, 1 - \text{end}(P_{2k}))$.
- Active inter-block edges connect $P_{2k+1}$ to $P_{2k+2 \pmod{2L}}$.

We maintain an inverse coordinate array:
$$pos(p) \in [0, 2L-1] \quad \forall p \in C_{giant}$$

### 2.2 Docking Port Identification
For a satellite cycle $C_i$, its docking ports on the Giant Cycle are:
$$\mathcal{D}(C_i) = \{ p \in C_{giant} \mid \exists q \in C_i, (u(p), u(q)) \in E(G) \setminus E_{current} \}$$
where $u(p) = \text{port\_to\_node}[p]$.

### 2.3 Optimal Circular Span with Boundary Buffers
1. For every docking port $d \in \mathcal{D}(C_i)$, get its index $pos(d)$.
2. Compute the circular interval $[pos_{start}, pos_{end}]$ that contains the densest cluster of docking ports with span $\le M_{max}$ blocks (default $M_{max} = 20$).
3. Extend the interval by buffer $\Delta = 2$ blocks on both sides:
   $$pos_{entry} = (pos_{start} - 2\Delta) \pmod{2L}, \quad pos_{exit} = (pos_{end} + 2\Delta) \pmod{2L}$$
4. The unfrozen corridor $\Omega$ is:
   $$\Omega = \text{Blocks}(C_i) \cup \text{Blocks}(C_{giant}[pos_{entry} \dots pos_{exit}])$$

### 2.4 The Two-Boundary Cut Invariant & Parity Soundness
- **Boundary:** Exactly 2 active edges cross the boundary between $\Omega$ and $V \setminus \Omega$:
  1. The entry edge entering $P_{entry}$ from $P_{entry - 1 \pmod{2L}}$.
  2. The exit edge leaving $P_{exit}$ to $P_{exit + 1 \pmod{2L}}$.
- **Internal Parity Conservation:**
  All blocks inside $\Omega$ are complete blocks (both endpoints present, internal degree-2 virtual edge enforced). Therefore:
  $$|A \cap \Omega| = |B \cap \Omega|$$
  Any valid 2-factor inside $\Omega$ must be a Hamiltonian path from $P_{entry}$ to $P_{exit}$ visiting all vertices of the Giant subpath and $C_i$.
  **Result:** Parity UNSAT is impossible.

---

## 3. Architecture & Interfaces

### 3.1 New Module: `src/cegar-fix/src/port_corridor_lns.rs`

```rust
pub struct PortCorridorLns;

impl PortCorridorLns {
    /// Sequentially absorbs satellite subcycles into the giant cycle using focused port corridors.
    /// Returns the repaired list of cycles (len == 1 if full Hamiltonian tour found).
    pub fn repair(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        encoder: &Encoder,
        base_cnf: &Cnf,
    ) -> Vec<Vec<i32>>;

    /// Attempts to absorb a single satellite cycle into the giant cycle.
    fn try_absorb_single_cycle(
        sat_cycle: &[Port],
        giant_cycle: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        encoder: &Encoder,
        base_cnf: &Cnf,
    ) -> Option<Vec<Port>>;
}
```

### 3.2 Corridor Assumption Generation
Inside `try_absorb_single_cycle`:
1. Collect all blocks in $\Omega$.
2. For every active edge $(u, v) \in E_{current}$:
   - If $u \notin \Omega$ and $v \notin \Omega \implies \text{assume } s_{u,v} = \text{True}$.
   - For all inactive neighbors $w$ of $u$ outside $\Omega \implies \text{assume } s_{u,w} = \text{False}$.
3. For the 2 boundary edges:
   - Assume $s_{prev, entry} = \text{True}$.
   - Assume $s_{exit, next} = \text{True}$.
   - Forbid all other boundary-crossing edges.
4. For all internal edges of $\Omega$: leave unconstrained for SAT solver exploration.

### 3.3 Corridor CEGAR Loop
Inside the local solver (fresh CaDiCaL instance initialized with `base_cnf` and mandatory degree-2 chain clauses):
```rust
for _cegar_iter in 0..8 {
    match local_solver.solve_assumps(&assumptions) {
        Ok(SolverResult::Sat) => {
            let model = local_solver.full_solution().unwrap();
            let arcs = extract_active_arcs(&model, encoder);
            let cycles = extract_cycles(arcs);
            let expected_len = giant_cycle.len() + sat_cycle.len();
            if let Some(merged) = cycles.iter().find(|c| c.len() == expected_len) {
                return Some(merged.clone());
            }
            // Add subcycle cuts for any cycle strictly internal to Omega
            let mut cut_added = false;
            for cyc in &cycles {
                if cyc.len() < expected_len && cyc.iter().all(|p| in_omega(p.block)) {
                    local_solver.add_clause(make_port_subcycle_cut(cyc, g, port_to_node, encoder));
                    cut_added = true;
                }
            }
            if !cut_added { break; }
        }
        _ => break,
    }
}
```

---

## 4. Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

In `cegar()` loop right after `AlternatingPortEngine::repair` (lines 583–587):
```rust
let mut sol_cycles = AlternatingPortEngine::repair(&sol_cycles, &g, contractor, 5, 2000);
if sol_cycles.len() > 1 && !contractor.chain_map.is_empty() {
    let absorbed = PortCorridorLns::repair(&sol_cycles, &g, contractor, encoder, &cnf);
    if absorbed.len() < sol_cycles.len() {
        println!("PortCorridorLns: absorbed satellite subcycles from {} down to {} cycles", sol_cycles.len(), absorbed.len());
        sol_cycles = absorbed;
    }
    if sol_cycles.len() == 1 && (sol_cycles[0].len() == g.adjacency_list.len() || sol_cycles[0].len() == contractor.original_vertices_count) {
        println!("*** 100% HAMILTONIAN TOUR FOUND VIA PORT CORRIDOR LNS! ***");
        let flat: Vec<i32> = sol_cycles.into_iter().flatten().collect();
        let final_tour = if flat.len() == contractor.original_vertices_count {
            flat
        } else {
            contractor.uncontract_cycle(&flat)
        };
        // Print solution and return
        return (count, clause_count, Some(final_tour));
    }
}
```

---

## 5. Verification & Soundness Invariants

1. **Zero Tour Injection:** No `.tou` reference files read anywhere.
2. **Zero Phantom Edges:** Every edge in the reconstructed tour must exist in `g.adjacency_list` (and raw uncontracted graph $G$).
3. **100% Soundness Certification:** Any generated tour must pass `scratch/verify_benchmarks.py`.
4. **Non-Regression:** All existing 55+ test suites must pass (`cargo test --tests`).
5. **Clean Build:** `cargo build --release` compiles with 0 errors.
