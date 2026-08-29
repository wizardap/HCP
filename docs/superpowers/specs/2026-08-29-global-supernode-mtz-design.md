# Design Specification: Global Supernode MTZ Potential Encoding (`GlobalSupernodeMTZ`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Global Order Challenge
In combinatorial HCP instances (`graph479.col`, `graph668.col`, `graph950.col`), pure CEGAR SEC cuts suffer from the combinatorial "whack-a-mole" problem: blocking $m$ local subcycles in Round $k$ simply prompts the solver to find a different partition of subcycles in Round $k+1$, requiring $> 20$ CEGAR iterations.
- **The Solution — `GlobalSupernodeMTZ`**:
  Partition the contracted graph ($N \sim 1800 \dots 2800$) into $K \in [12, 20]$ balanced supernodes (macro-regions of size $\sim 150 \dots 200$ vertices).
  Encode Miller-Tucker-Zemlin (MTZ) unary potentials across the $K$ supernodes directly into the base CNF at Round 0:
  - Unary order variables: $K \times (K - 1) \le 380$ variables.
  - MTZ transition clauses: $O(K \cdot |E_{\text{meta}}|) \le 2,500$ clauses.
  - Overhead: $< 10\text{ms}$ encoding time, negligible SAT memory footprint.
  - **Guarantee**: Mathematically forbids ANY macro-subcycle across the $K$ supernodes, forcing CaDiCaL to search exclusively within the subspace of single, globally connected Hamiltonian paths traversing all $K$ supernodes!

---

## 2. Mathematical Formulation & Architecture

### 2.1 Balanced Supernode Partitioning
1. Given graph $G = (V, E)$ with $|V| = N$:
   - Target $K = 16$ supernodes $\implies \text{target\_size} = \max(25, N / 16)$.
   - Partition $V$ into connected modules $M_0, M_1, \dots, M_{K-1}$.
2. **Unary Order MTZ Encoding**:
   - For each module $i \in \{0, \dots, K-1\}$ and step $t \in \{1, \dots, K-1\}$, allocate unary variable $O_{i, t}$ ($O_{i, t} \iff \text{order}(M_i) \ge t$).
   - **Monotonicity**: $\neg O_{i, t} \lor O_{i, t-1}$ for all $t \ge 2$.
   - **Root Anchor**: $\neg O_{0, 1}$ (Module $M_0$ has order 0).
3. **Meta-Edge Transition Implications**:
   - For each directed meta-edge $(M_i, M_j)$ with boundary edge literals $L_{ij} = \{l_{uv} \mid u \in M_i, v \in M_j\}$:
     - Allocate meta-edge indicator $X_{ij}$ with clauses: $\neg l_{uv} \lor X_{ij}$ for each $l_{uv} \in L_{ij}$.
     - When $X_{ij} = 1$ and $j \neq 0$:
       - $\neg X_{ij} \lor O_{j, 1}$ ($M_j$ must have order $\ge 1$).
       - $\neg X_{ij} \lor \neg O_{i, t} \lor O_{j, t+1}$ for all $1 \le t < K - 1$ (if order($M_i$) $\ge t$, then order($M_j$) $\ge t+1$).
       - $\neg X_{ij} \lor \neg O_{i, K-1}$ ($M_i$ cannot have order $K-1$).
4. **Boundary Cut Clauses**:
   - For every module $M_i$: at least 1 outgoing boundary edge ($\bigvee_{v \in \delta^+(M_i)} l_{uv}$) and at least 1 incoming boundary edge ($\bigvee_{u \in \delta^-(M_i)} l_{uv}$).

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` at Round 0:
```rust
// Global Supernode MTZ Potential Encoding
if g.adjacency_list.len() >= 50 {
    let target_k = 16;
    let target_size = (g.adjacency_list.len() / target_k).max(25);
    let modules = MetagraphRouter::detect_gadget_modules_with_size(&g, target_size);
    if modules.len() >= 4 && modules.len() <= 24 {
        println!("GlobalSupernodeMTZ: generated {} supernodes (target size {}), injecting global MTZ order encoding", modules.len(), target_size);
        MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);
    }
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_metagraph_router.rs`):**
   - Test balanced $K=16$ partitioning on synthetic 200-vertex and 500-vertex graphs.
   - Test subcycle prevention across supernodes with MTZ encoding.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test complete `solve_hamilton` with `GlobalSupernodeMTZ` enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
