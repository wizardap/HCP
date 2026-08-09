# Plan: Parallel Cluster Sub-HCP Solving (Adaptive Level 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Parallel Cluster Sub-HCP Solving as Adaptive Escalation Level 3 in `cegar-fix`. When local search stalls at Level 2 (`stall_count >= 9`), partition subcycles into independent clusters, create fresh mini-HCP problems on induced subgraphs, and solve them concurrently using multi-threading.

## Tasks

- [ ] **Task 1: Add CLI Flags & Methods for Induced Subgraphs**
  - Files: `src/cegar-fix/src/options.rs`, `src/cegar-fix/src/main.rs`, `src/cegar-fix/src/graph.rs`
  - Action: 
    - Add `--sub-hcp-timeout` (default: 60) and `--max-cluster-size` (default: 500) flags in `options.rs` & `main.rs`.
    - Add `induced_subgraph(&self, vertices: &HashSet<i32>) -> Graph` in `graph.rs`.

- [ ] **Task 2: Implement Clustering & Parallel Sub-HCP Solver Module**
  - Files: `src/cegar-fix/src/parallel_sub_hcp.rs`, `src/cegar-fix/src/main.rs` (module declaration)
  - Action:
    - Implement `build_subcycle_adjacency_graph` and greedy Union-Find clustering algorithm with size caps.
    - Implement `solve_cluster_sub_hcp` (fresh `Encoder` + `CaDiCaL` mini-CEGAR loop on induced subgraph).
    - Implement `solve_parallel_clusters` using `std::thread::spawn` and result aggregation.

- [ ] **Task 3: Wire Level 3 Trigger into `hcp_solver.rs` CEGAR Loop**
  - Files: `src/cegar-fix/src/hcp_solver.rs`
  - Action:
    - Thread new flags (`sub_hcp_timeout`, `max_cluster_size`) into `solve_hamilton` and `cegar`.
    - In `cegar`, when `adaptive_escalation == 1` and `stall_count >= 9`, trigger Level 3:
      - Call `solve_parallel_clusters`.
      - Apply merged cycles back into active cycles.
      - Reset `stall_count` if progress was made.

- [ ] **Task 4: Build, Verify and Benchmark**
  - Files: N/A
  - Action:
    - Run `cargo check` and `cargo build --release` in `src/cegar-fix/`.
    - Benchmark on `graph12.col` (verify Level 3 does not trigger and speed is < 0.2s).
    - Benchmark on `graph998.col` or `graph339.col` to verify Level 3 clustering, parallel thread execution, and cycle merging.
