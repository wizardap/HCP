# Design Specification: Dual-Channel Metagraph Router

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Dual-Channel Gadget Discovery
In empirical testing of `MetagraphRouter`, CaDiCaL proved single-supernode MTZ unsatisfiable in 19.4ms because:
1. **Gadgets Require Two Disjoint Internal Paths:** In FHCP Challenge instances (e.g. `graph479.col`, `graph668.col`), each 22-vertex gadget module cannot be covered by a single Hamiltonian path entering and exiting once.
2. **Two Independent 11-Vertex Channels:** The 22 vertices inside each gadget are split into two disjoint 11-vertex internal paths (Channel 0 and Channel 1).
3. **The Global Tour Weaves Twice:** The global Hamiltonian cycle visits Channel 0 during the traversal of Hemisphere 1, and visits Channel 1 during the traversal of Hemisphere 2.
4. **The Solution — Dual-Channel Supernode MTZ:**
   - Partition each module $M_k$ into two sub-channels $C_{k, 0}$ and $C_{k, 1}$.
   - Encode MTZ order constraints across the $K_{\text{chan}} = 2 \times K_{\text{modules}}$ channel supernodes.
   - Because each channel is visited **exactly once**, the formulation is 100% satisfiable, sound, and guarantees a single unified Hamiltonian cycle across all channels.

---

## 2. Mathematical Formulation & Architecture

### 2.1 Channel Supernode Decomposition
- Each module $M_k$ is partitioned into two disjoint channels $C_{k, 0}$ and $C_{k, 1}$ using internal BFS / 2-cut from the distinct boundary interface nodes.
- Total channel supernodes: $\mathcal{V}_{\text{chan}} = \{0, 1, \dots, 2K - 1\}$.
- For each channel $i$, its boundary edges $\partial(C_i)$ connect to channels in other modules.

### 2.2 Unary Order MTZ Formulation on 168 Channels
- Order variables: $u_i \in \{0, 1, \dots, 2K - 1\}$ encoded via ladder variables $O_{i, t} \iff (u_i \ge t)$ for $1 \le t < 2K$.
- Monotonicity: $\neg O_{i, t} \lor O_{i, t-1}$ for all $2 \le t < 2K$.
- Root fixing: Channel 0 is root ($u_0 = 0 \implies \neg O_{0, 1}$).
- Meta-edge variables $X_{ij}$: For each directed channel pair $(i, j)$ with $i \neq j$:
  - For each underlying edge $l_{uv} \in \partial(C_i, C_j)$: $\neg l_{uv} \lor X_{ij}$.
  - If $j \neq 0$:
    - $\neg X_{ij} \lor O_{j, 1}$
    - For $1 \le t < 2K - 1$: $\neg X_{ij} \lor \neg O_{i, t} \lor O_{j, t+1}$
    - $\neg X_{ij} \lor \neg O_{i, 2K-1}$

---

## 3. Module & File Architecture

### 3.1 `src/cegar-fix/src/metagraph_router.rs`
```rust
#[derive(Debug, Clone)]
pub struct ChannelModule {
    pub id: usize,
    pub parent_gadget_id: usize,
    pub channel_idx: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>,
}

impl MetagraphRouter {
    pub fn detect_dual_channels(g: &Graph) -> Vec<ChannelModule>;
    pub fn encode_dual_channel_mtz(
        channels: &[ChannelModule],
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    );
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_metagraph_router.rs`):**
   - Test dual-channel module decomposition on 2-channel gadget chains.
   - Test dual-channel MTZ encoding satisfiability on valid tours vs. UNSAT on 2-factor subcycles.
2. **Integration Test (`tests/test_staged_solver.rs`):**
   - Test complete CEGAR loop with `DualChannelRouter`.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
