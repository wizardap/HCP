# Task 1 Brief: `MetagraphRouter` Engine

## Overview
Implement the `MetagraphRouter` engine in `src/cegar-fix/src/metagraph_router.rs` to automatically partition graphs into supernode modules and encode Miller-Tucker-Zemlin (MTZ) unary order constraints on the supernodes.

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Commands use `taskset -c 0,1,2 nice -n 19` (Core 3 reserved for user).
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.

## Requirements & Interfaces

### 1. File Structure
- Create: `src/cegar-fix/src/metagraph_router.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod metagraph_router;`)
- Test: `src/cegar-fix/tests/test_metagraph_router.rs`

### 2. Interface Specification
```rust
#[derive(Debug, Clone)]
pub struct GadgetModule {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>, // directed edges (u, v) with u in this module, v in another module
}

pub struct MetagraphRouter;

impl MetagraphRouter {
    /// Partitions the graph into gadget modules (connected clusters of vertices).
    pub fn detect_gadget_modules(g: &Graph) -> Vec<GadgetModule>;

    /// Encodes MTZ unary order constraints across all supernodes into cnf.
    pub fn encode_supernode_mtz(
        modules: &[GadgetModule],
        g: &Graph,
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    );
}
```

### 3. Algorithm Details
1. **`detect_gadget_modules`**:
   - Compute connected clusters:
     - Group vertices based on community / partition or BFS with max module size (e.g. 20-30 vertices).
     - Assign each vertex $v \in V$ to a `module_map: HashMap<i32, usize>`.
     - For each module $i$, gather all directed edges $(u, v) \in E(G)$ where $u \in M_i$ and $v \notin M_i$ into `boundary_edges`.
2. **`encode_supernode_mtz`**:
   - Let $K = \text{modules.len()}$. If $K \le 2$, return.
   - For each module $i \in \{0, \dots, K-1\}$ and step $t \in \{1, \dots, K-1\}$, allocate a Boolean order literal $O_{i, t} \iff (u_i \ge t)$ using `encoder.next_variable()`.
   - Add order monotonicity: $\neg O_{i, t} \lor O_{i, t-1}$ for all $2 \le t < K$.
   - Root fixing for module 0: $u_0 = 0 \implies \neg O_{0, 1}$.
   - For each directed meta-edge between module $i$ and module $j$ ($i \neq j$):
     - Meta-edge indicator $X_{ij}$:
       For each boundary edge $(u, v)$ from $M_i$ to $M_j$, get literal $l_{uv} = \text{encoder.graph\_lit\_map.get}(\&(u, v))$.
       If $j \neq 0$:
         For each $l_{uv}$:
           - $\neg l_{uv} \lor O_{j, 1}$
           - For $1 \le t < K - 1$: $\neg l_{uv} \lor \neg O_{i, t} \lor O_{j, t+1}$
           - $\neg l_{uv} \lor \neg O_{i, K-1}$

### 4. Unit Tests in `src/cegar-fix/tests/test_metagraph_router.rs`
- `test_detect_gadget_modules`: Partition a 3-module graph and verify module count and boundary edges.
- `test_encode_supernode_mtz`: Verify that supernode MTZ clauses are generated and eliminate subcycles on a metagraph.
