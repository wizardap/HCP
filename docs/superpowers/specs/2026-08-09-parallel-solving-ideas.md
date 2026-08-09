# Parallel Solving Ideas for CEGAR HCP Solver

**Date**: 2026-08-09  
**Status**: Brainstorming (Not yet designed)

---

## Bottleneck Analysis (from graph998.col benchmark)

| Component | % Runtime | Notes |
|---|---|---|
| SAT Solving (`solver.solve()`) | **99.5%** | Single CaDiCaL call takes 30s-475s per iteration |
| Clause Generation (2-opt/3-opt/blocking) | 0.5% | ~0.35s-0.55s per iteration |

## Architectural Constraints
- CEGAR iterations are strictly sequential (iteration N+1 depends on N's SAT result)
- `encoder.instance` is mutated when creating new variables (MTZ, balanced encoding)
- CaDiCaL supports `Send` but NOT `Sync` — each thread needs its own instance
- `graph_lit_map` is read-only after initial encoding

---

## Idea 1: Graph Partitioning (Divide & Conquer on Graph Structure)
- Partition graph G into k sub-graphs
- Find Hamiltonian paths in each sub-graph in parallel
- Stitch paths into a global Hamiltonian cycle
- **Challenge**: Boundary vertex selection, stitching correctness, sub-problems still NP-hard

## Idea 2: Subtour Partitioning (Parallel Subcycle Merging) ← SELECTED FOR EXPLORATION
- When SAT solver returns ~500 subcycles, partition into k groups
- Run 2-opt/3-opt merging independently on each group in parallel
- Merge results across groups
- **Potential benefit**: Faster convergence by exploring more merge combinations

## Idea 3: Search Space Splitting (Cube-and-Conquer)
- Split SAT search space into k cubes (partial variable assignments)
- Each thread solves one cube with its own CaDiCaL instance
- First thread to find SAT terminates all others
- **Benefit**: Directly attacks the 99.5% bottleneck (SAT solving time)
- **Challenge**: Requires look-ahead solver for cube generation; cube quality is critical

## Idea 4: Portfolio Parallel SAT Solving
- Run N CaDiCaL instances with different configs/seeds on same CNF
- First to finish wins
- **Benefit**: Simple to implement, directly attacks SAT bottleneck
- **Challenge**: Diminishing returns with many instances; memory overhead

## Idea 5: Speculative CEGAR Branching
- Fork into N CEGAR branches, each trying different blocking strategies
- Branch that finds Hamilton cycle first wins
- **Challenge**: Each branch needs its own solver+encoder copy; memory heavy
