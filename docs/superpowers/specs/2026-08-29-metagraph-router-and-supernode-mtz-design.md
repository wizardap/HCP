# Design Specification: Metagraph Router & Supernode MTZ Encoding

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 Root-Cause Analysis on 84-Gadget Macro-Oscillations
Empirical analysis of 1800s runs on `graph479.col` ($N=1,848$) and `graph668.col` ($N=2,862$) revealed:
1. **Intra-Gadget Routing is 100% Solved:** All subcycles in later CEGAR rounds have lengths that are exact multiples of 22 (e.g. 220, 308, 484, 704, 924, 1540), proving that CaDiCaL effortlessly finds valid Hamiltonian paths inside all 84 individual 22-vertex gadgets.
2. **The 2-Factor CEGAR Combinatorial Trap:** Standard 2-factor local degree encodings have no global connectivity awareness. With $2^{84}$ interface states, CEGAR gets trapped adding thousands of blocking clauses one-by-one to rule out exponentially many macro-permutations (e.g. $\{924, 924\}$, $\{308, 1540\}$).
3. **The Solution — Supernode MTZ Encoding:** 
   - While full-vertex MTZ on 1,848 vertices requires $\sim 3.4 \times 10^6$ clauses (untenable), MTZ on the **84 Super-Nodes** requires only $\sim 14,000$ binary clauses.
   - Injecting 84-supernode MTZ at Round 0 guarantees 100% global connectivity across the entire metagraph, completely eliminating macro-subcycles before solving begins.

---

## 2. Mathematical Formulation & Architecture

### 2.1 Metagraph Representation
- Partition $V(G')$ into disjoint gadget modules $\mathcal{M} = \{M_0, M_1, \dots, M_{K-1}\}$ ($K \approx 84$).
- Define directed meta-graph $\mathcal{G}_{\text{meta}} = (\mathcal{V}_{\text{meta}}, \mathcal{E}_{\text{meta}})$ where:
  - $\mathcal{V}_{\text{meta}} = \{0, 1, \dots, K-1\}$
  - $(i, j) \in \mathcal{E}_{\text{meta}} \iff \exists u \in M_i, v \in M_j \text{ such that } (u, v) \in E(G')$.
- Meta-edge indicator variable: $X_{ij} = \bigvee_{u \in M_i, v \in M_j, (u, v) \in E} x_{uv}$.

### 2.2 Unary Order MTZ Formulation on Supernodes
- Order variables $u_i \in \{0, 1, \dots, K-1\}$ encoded using unary order variables $O_{i, t}$ ($1 \le t < K$):
  $$O_{i, t} \iff (u_i \ge t)$$
- Order monotonicity:
  $$\neg O_{i, t} \lor O_{i, t-1} \quad \forall 2 \le t < K$$
- Root fixing: $u_0 = 0 \implies \neg O_{0, 1}$.
- Transitive Step / Subtour Elimination:
  For each directed meta-edge $(i, j) \in \mathcal{E}_{\text{meta}}$ with $j \neq 0$:
  $$X_{ij} \implies (u_j \ge u_i + 1)$$
  In CNF clauses:
  $$\neg X_{ij} \lor \neg O_{i, t} \lor O_{j, t+1} \quad \forall 1 \le t < K - 1$$
  $$\neg X_{ij} \lor \neg O_{i, K-1} \quad (\text{cannot step beyond max order})$$
  $$\neg X_{ij} \lor O_{j, 1} \quad (\text{any target } j \neq 0 \text{ must have order } \ge 1)$$

---

## 3. Module & File Architecture

### 3.1 `src/cegar-fix/src/metagraph_router.rs`
```rust
pub struct GadgetModule {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>,
}

pub struct MetagraphRouter;

impl MetagraphRouter {
    pub fn detect_gadget_modules(g: &Graph) -> Vec<GadgetModule>;
    pub fn encode_supernode_mtz(
        modules: &[GadgetModule],
        g: &Graph,
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    );
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_metagraph_router.rs`):**
   - Test module detection on synthetic gadget chains and ladders.
   - Test supernode MTZ clause validity and subcycle prevention on meta-graphs.
2. **Integration Test (`tests/test_staged_solver.rs`):**
   - Test complete CEGAR loop with `MetagraphRouter` enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
