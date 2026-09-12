# Design Specification: Cluster-Anchor Window Extraction & Unary MTZ Topological Ordering for Port-Corridor LNS

**Date:** 2026-09-12  
**Status:** Approved for Implementation  
**Target:** `src/cegar-fix/src/port_corridor_lns.rs`  
**Related Documents:**
- Plan: [`docs/superpowers/plans/2026-09-12-port-corridor-lns.md`](file:///home/ubuntu/HCP/docs/superpowers/plans/2026-09-12-port-corridor-lns.md)
- Phenomenon Report: [`docs/research/phenomena/2026-09-12-graph868-corridor-parity-cliff-phenomenon.md`](file:///home/ubuntu/HCP/docs/research/phenomena/2026-09-12-graph868-corridor-parity-cliff-phenomenon.md)
- Benchmark Telemetry: [`scratch/log_graph868_corridor_1800s.txt`](file:///home/ubuntu/HCP/scratch/log_graph868_corridor_1800s.txt)

---

## 1. Executive Summary & Root Cause Motivation

During the 1,800-second benchmark on `FHCPCS-col/graph868.col`, the `PortCorridorLns` engine achieved an unprecedented breakthrough by **eliminating the Bipartite Parity Barrier**: local CaDiCaL instances on a 26-block corridor $\Omega$ solved in **6.36 milliseconds** ($Ok(Sat)$).

However, full Hamiltonian absorption was obstructed by two structural phenomena:
1. **Internal 2-Block Subcycle Shattering**:
   Because CaDiCaL was only constrained by degree-2 and boundary cuts, it satisfied degree-2 requirements inside $\Omega$ by synthesizing detached 2-block cycles (e.g. Cycle 18: blocks 1 and 1208, 4 ports) alongside a short path between $P_{entry}$ and $P_{exit}$, rather than a unified Hamiltonian subpath visiting all blocks in $\Omega$.
2. **Docking Anchor Misalignment**:
   The original corridor extractor tried to find a contiguous interval covering *all* docking ports of a satellite cycle (which are scattered over the 3,372-port Giant ring), and then clamped the interval to `max_span = 20`. This truncated the window, leaving only a single docking edge to the satellite cycle in the extracted corridor, rendering 2-port entry/exit absorption topologically impossible.

This specification designs:
1. **Cluster-Anchor Window Extraction**: Identifies pairs of docking ports $(d_1, d_2)$ with minimal span ($\le 30$ blocks) on the Giant Cycle having complementary cross edges to the satellite cycle.
2. **Unary MTZ Topological Ordering**: Injects a lightweight propositional order ladder ($O_{b, t}$) directly into the local CaDiCaL instance, mathematically forbidding any closed internal subcycle within $\Omega$ and forcing CaDiCaL to find a strictly simple Hamiltonian path.

---

## 2. Mathematical Formulation & Clausal Encoding

### 2.1 Cluster-Anchor Window Extraction

Given:
- Giant Cycle $C_{giant} = [P_0, P_1, \dots, P_{2L-1}]$ where external edges connect $2k \leftrightarrow 2k+1$ and internal chains connect $2k+1 \leftrightarrow (2k+2) \bmod 2L$.
- Satellite Cycle $C_{sat}$.
- Docking ports $D = \{ p \in C_{giant} \mid \exists u \in node(p), v \in node(C_{sat}) \text{ with } (u, v) \in E(G) \}$.

**Algorithm**:
1. Map each docking port $p \in D$ to its coordinate $pos[p] \in [0, 2L-1]$.
2. For every pair of docking ports $(d_1, d_2) \in D \times D$ ($d_1 \ne d_2$):
   - Calculate forward distance $d_{fwd} = (pos[d_2] + 2L - pos[d_1]) \bmod 2L$.
   - Calculate backward distance $d_{bwd} = (pos[d_1] + 2L - pos[d_2]) \bmod 2L$.
   - Let $span = \min(d_{fwd}, d_{bwd})$.
   - If $span \le max\_span \times 2$ (default $30 \times 2 = 60$ ports) and $d_1, d_2$ connect to distinct blocks in $C_{sat}$:
     - Record candidate anchor $(d_{start}, d_{end}, span)$.
3. Sort candidate anchors ascending by $span$.
4. For each candidate anchor, compute parity-aligned boundaries:
   - Add buffer $buf\_ports = buffer \times 2$ (default $2 \times 2 = 4$).
   - $raw\_start = (pos[d_{start}] + 2L - buf\_ports) \bmod 2L$.
   - $entry\_idx = \text{if } raw\_start \bmod 2 == 0 \text{ then } (raw\_start + 2L - 1) \bmod 2L \text{ else } raw\_start$ (strictly odd, $2k+1$).
   - $raw\_end = (pos[d_{end}] + buf\_ports) \bmod 2L$.
   - $exit\_idx = \text{if } raw\_end \bmod 2 \ne 0 \text{ then } (raw\_end + 1) \bmod 2L \text{ else } raw\_end$ (strictly even, $2m$).
   - $total\_subpath\_ports = \text{if } exit\_idx \ge entry\_idx \text{ then } exit\_idx - entry\_idx + 1 \text{ else } (2L - entry\_idx) + exit\_idx + 1$.
   - Invariant: $total\_subpath\_ports$ is strictly even, guaranteeing $|A \cap \Omega| = |B \cap \Omega|$.

---

### 2.2 Unary MTZ Topological Ordering Encoding

Let $\Omega$ be the set of unfrozen blocks:
$$\Omega = blocks(C_{sat}) \cup giant\_subpath\_blocks, \quad K = |\Omega| \le 30$$

Every block $b \in \Omega$ must appear at a unique discrete topological step $t \in [0, K-1]$ along the alternating path from $P_{entry}$ to $P_{exit}$.

#### 1. Variable Semantics
For each block $b \in \Omega$ and position $t \in [1, K-1]$:
- $O_{b, t} \in \{0, 1\}$: True if and only if block $b$ is visited at step $\ge t$.
- Variable allocation: $K \times (K-1) \le 30 \times 29 = 870$ fresh Boolean variables.

#### 2. Ladder Consistency Clauses (Order Invariants)
A ladder variable cannot be true if its predecessor is false:
$$\forall b \in \Omega, \forall t \in [2, K-1]: \quad O_{b, t} \implies O_{b, t-1} \quad \equiv \quad (\neg O_{b, t} \lor O_{b, t-1})$$
Clause count: $K \times (K-2) \approx 30 \times 28 = 840$ binary clauses.

#### 3. Boundary Position Anchors
- The entry block $b_{entry} = corridor.entry\_port.block$ is fixed at step 0:
  $$\neg O_{b_{entry}, 1}$$
- The exit block $b_{exit} = corridor.exit\_port.block$ is fixed at step $K-1$:
  $$O_{b_{exit}, K-1}$$
- No non-exit block can be at step $K-1$:
  $$\forall b \in \Omega \setminus \{b_{exit}\}: \quad \neg O_{b, K-1}$$

#### 4. Step Transition Implication (Topological MTZ Propagation)
For every candidate external edge $e = (u, v)$ where $u \in node(P(b_1, e_1))$, $v \in node(P(b_2, e_2))$ with $b_1, b_2 \in \Omega, b_1 \ne b_2$:
If directed edge $u \to v$ is selected ($x_{u \to v} = 1$), then $b_2$ must be visited at least 1 step after $b_1$:
$$\forall t \in [1, K-2]: \quad x_{u \to v} \land O_{b_1, t} \implies O_{b_2, t+1}$$
In CNF format:
$$(\neg x_{u \to v} \lor \neg O_{b_1, t} \lor O_{b_2, t+1})$$
Also for base step $t = 0$ (if $b_1$ is at step $\ge 0$, which is always true, $b_2$ must be at step $\ge 1$):
$$(\neg x_{u \to v} \lor O_{b_2, 1})$$

Clause count: For $\approx 50$ candidate external edges in $\Omega$, each produces $K-1 \approx 29$ clauses $\approx 1,450$ 3-literal clauses.
Total clauses: $\approx 2,300$ clauses.

#### 5. Theoretical Guarantee
Any cycle $b_{i_1} \to b_{i_2} \to \dots \to b_{i_m} \to b_{i_1}$ inside $\Omega$ requires:
$$O_{b_{i_1}} < O_{b_{i_2}} < \dots < O_{b_{i_m}} < O_{b_{i_1}}$$
which is unsatisfiable by BCP unit propagation. **Zero internal subcycles are possible.**

---

## 3. Rust Architectural Interface

In `src/cegar-fix/src/port_corridor_lns.rs`:

```rust
pub struct AnchorCandidate {
    pub start_pos: usize,
    pub end_pos: usize,
    pub span: usize,
    pub sat_dock1: Port,
    pub sat_dock2: Port,
}

impl PortCorridorLns {
    /// Discovers all candidate docking anchor windows with span <= max_span.
    pub fn find_anchor_candidates(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        max_span: usize,
    ) -> Vec<AnchorCandidate>;

    /// Builds a PortSubpathCorridor anchored at a specific candidate pair.
    pub fn build_corridor_from_anchor(
        anchor: &AnchorCandidate,
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        buffer: usize,
    ) -> Option<PortSubpathCorridor>;

    /// Injects Unary MTZ ladder variables and ordering clauses into local CaDiCaL.
    pub fn inject_unary_mtz_ordering(
        solver: &mut CaDiCaL,
        corridor: &PortSubpathCorridor,
        encoder: &Encoder,
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        next_free_var: &mut i32,
    ) -> (HashMap<(usize, usize), Lit>, usize);
}
```

---

## 4. Invariants & Proofs

### Invariant 1: Soundness
- Any satisfying assignment inside $\Omega$ must satisfy the Unary MTZ ladder clauses.
- Because every external transition enforces strict step advancement ($O_{b_2} \ge O_{b_1} + 1$), no subset of blocks $S \subseteq \Omega$ can form an isolated directed cycle.
- Because all outside vertices are frozen to active external/internal edges and boundary edges $e_{in}, e_{out}$ connect strictly to $P_{entry}$ and $P_{exit}$, the reconstructed tour is a single, valid, closed Hamiltonian cycle.

### Invariant 2: Completeness
- If an alternating Hamiltonian splicing of $C_{sat}$ into the giant subpath exists within $\Omega$, it defines a topological order $0, 1, \dots, K-1$ of the blocks in $\Omega$.
- Assigning $O_{b, t} = \text{True} \iff order(b) \ge t$ satisfies all ladder, boundary, and transition clauses without contradiction.

### Invariant 3: Zero Tour Injection
- No `.tou` files or precomputed solutions are read or referenced.
- All candidate edges are strictly verified against `g.adjacency_list`.

---

## 5. Verification & Testing Strategy

1. **Unit Test: Synthetic Gadget Absorption (`test_corridor_cluster_mtz_unit`)**:
   - 6 blocks ($N=12$), two 2-block cycles connected to a 4-block giant subpath.
   - Verify that MTZ prevents isolated 2-block cycle formation and finds the single unified tour.
2. **Multi-Anchor Selection Test (`test_anchor_candidate_selection`)**:
   - Verify that widely separated docking ports on a ring are rejected in favor of dense minimal-span pairs.
3. **Snapshot Verification (`test_graph868_snapshot_mtz_reduction`)**:
   - Test on `scratch/graph868_giant_1686_full_edges.txt`.
   - Verify that CaDiCaL solves the MTZ-constrained corridor in $< 20$ms and absorbs satellite cycles.
4. **Full Workspace Regression**:
   - `cargo test --tests` must pass 100% across all 56 targets.
   - `cargo build --release` with 0 warnings/errors.
