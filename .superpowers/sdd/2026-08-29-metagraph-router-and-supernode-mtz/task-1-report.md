# Task 1 Report: `MetagraphRouter` Engine & Supernode MTZ Encoder

## 1. Executive Summary
- **Module**: `src/cegar-fix/src/metagraph_router.rs`
- **Exports**: Added to `src/cegar-fix/src/lib.rs` and `src/cegar-fix/src/main.rs`
- **Tests**: `src/cegar-fix/tests/test_metagraph_router.rs` (5 comprehensive unit tests, all passing)
- **Git Commit**: `526d01d` (`feat(metagraph): implement metagraph router and supernode MTZ encoder`)

---

## 2. Implementation Details

### `GadgetModule` Structure
```rust
#[derive(Debug, Clone)]
pub struct GadgetModule {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>, // directed edges (u, v) where u in module, v outside module
}
```

### `MetagraphRouter::detect_gadget_modules` & `detect_gadget_modules_with_size`
1. Computes edge neighborhood intersections (`shared_neighbors`) to detect strong intra-module clustering bonds.
2. Partitions graph into cohesive modules (components of strong edges or connected components), with BFS subdivision ensuring maximum module size bounds ($\le 25$ vertices).
3. Deterministically sorts clusters by minimum vertex ID and indexes each vertex into `vertex_to_module`.
4. Computes directed `boundary_edges` for each module $(u, v) \in E(G)$ where $u \in M_i$ and $v \notin M_i$, deduplicating and sorting them.

### `MetagraphRouter::encode_supernode_mtz`
1. Early return if $K \le 2$ (no-op).
2. Allocates unary order ladder variables $O_{i, t}$ ($t \in \{1, \dots, K-1\}$) for each module $i \in \{0, \dots, K-1\}$.
3. Enforces ladder monotonicity: $\neg O_{i, t} \lor O_{i, t-1}$ for all $2 \le t < K$.
4. Enforces root module order fixing: $u_0 = 0 \implies \neg O_{0, 1}$.
5. Enforces MTZ order progression across inter-module directed boundary edges $(u, v)$ with $u \in M_i, v \in M_j$ ($j \ne 0$):
   - $\neg l_{uv} \lor O_{j, 1}$
   - For $1 \le t < K-1$: $\neg l_{uv} \lor \neg O_{i, t} \lor O_{j, t+1}$
   - $\neg l_{uv} \lor \neg O_{i, K-1}$
6. Enforces supernode boundary entry/exit cut clauses: at least 1 incoming and 1 outgoing boundary edge per module for $K \ge 3$.

---

## 3. Test Verification
All unit tests in `src/cegar-fix/tests/test_metagraph_router.rs` and the entire test suite pass cleanly:
- `test_detect_gadget_modules`: Verified 3-module graph partitioning into exact disjoint modules with boundary edges.
- `test_encode_supernode_mtz`: Verified that disconnected 3-module subcycles are proved UNSAT, while valid 9-cycle Hamiltonian tour is SAT.
- `test_small_k_supernode_mtz_no_op`: Verified $K \le 2$ generates 0 clauses.
- `test_empty_graph_and_single_module`: Verified edge cases for empty and 1-module graphs.
- `test_encode_supernode_mtz_four_modules_subcycles`: Verified 4-module subcycle elimination and full tour SAT solvability.

Full test suite: 77 tests run across all modules in `cegar-fix`, 100% passing with 0 regressions.

---

## 4. Status Contract
- **Status**: DONE
- **Commits created**: `526d01d`
- **Test Summary**: 5/5 unit tests passed in `test_metagraph_router.rs`; 77/77 tests passed across entire test suite.
- **Concerns**: None.
- **Report File**: `/home/ubuntu/HCP/.superpowers/sdd/2026-08-29-metagraph-router-and-supernode-mtz/task-1-report.md`
