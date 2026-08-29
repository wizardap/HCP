# Design Specification: Extended Static Cycle & Gadget Perimeter Cutter up to Length 16 (`StaticCycleCutter`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Root Cause

### 1.1 The 16-Cycle Gadget Perimeter Trap
In 3-SAT-derived HCP graphs (`graph479.col`, `graph668.col`, `graph950.col`), each gadget module contains a 16-vertex cyclic perimeter.
- **The Failure Mode**: A valid Hamiltonian tour must traverse the gadget's internal Hamiltonian path from port to port. However, a 2-factor can short-circuit the entire gadget by choosing the 16 perimeter edges as an isolated 16-cycle, leaving the rest of the tour intact.
- **Combinatorial Explosion**: With $\sim 60$ gadgets, there are $2^{60}$ distinct combinations of 16-cycles. Adding block clauses reactively in CEGAR allows CaDiCaL to simply switch to another subset of gadgets every round, stalling the solver for $> 20$ rounds.
- **The Solution**: Extend `StaticCycleCutter` at Round 0 to detect all induced cycles of length up to 16 ($9 \dots 16$, especially 16-cycle perimeters) and inject bidirectional subtour elimination clauses:
  $$\bigvee_{i=1}^k \neg e_{v_i, v_{i+1}}$$
  at Round 0 in $< 200\text{ms}$. This statically eliminates all $2^{60}$ gadget short-circuits before CDCL begins.

---

## 2. Algorithmic Design

### 2.1 Bounded DFS / Chordless Cycle Enumeration in `StaticCycleCutter`
```rust
impl StaticCycleCutter {
    /// Detects chordless induced cycles of lengths 9..=16 with budget caps
    /// and generates directional subtour elimination clauses.
    pub fn generate_extended_cycle_cuts(
        g: &Graph,
        encoder: &Encoder,
        min_len: usize, // e.g. 9
        max_len: usize, // e.g. 16
        max_clauses: usize, // e.g. 15000
    ) -> Cnf;
}
```

### 2.2 Chordless Validation & Efficiency
- Since max degree in compressed gadget graphs is $\le 3$, a depth-bounded DFS with early pruning on visited vertices and canonical ordering ($u_1 = \min_{v \in C} v, u_2 < u_k$) runs in $< 100\text{ms}$.
- Verify that the cycle contains no chord edges $(v_i, v_j) \in E(G)$ with $|i - j| > 1 \pmod k$.
- Generate directional clauses for both clockwise and counter-clockwise traversals.

---

## 3. Integration into `hcp_solver.rs`
- In `hcp_solver.rs` at Round 0:
  `let static_cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);`
  combines 3-, 4-, 6-, 7-, 8-, and 9..=16-cycle static clauses.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_static_cycle_cutter.rs`):**
   - Test detection of induced 12-, 14-, and 16-cycles on synthetic gadget rings.
   - Test chord pruning (cycles with chords are skipped or handled).
   - Test directional clause validity in `Encoder`.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR solver with extended static cuts enabled on gadget graphs.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
