# Pure-Rust Topological Generalization Design Specification

## Overview
This design eliminates all hardcoded vertex counts, graph IDs, and hardcoded vertex indices from the pure-Rust HCP solver (`src/cegar-fix`). In their place, the solver introduces **Dynamic Topological Feature Probes** that inspect graph characteristics (articulation pairs, degree distributions, degree-2 chain structures) and dispatch the appropriate structural macro or fallback solver automatically.

## Architectural Changes

### 1. `macro_decomp/corridor.rs`
- **Current State:**
  - Contains `solve_710(&g, timeout)` which enforces `raw_g.adjacency_list.len() == 4064`.
- **Target State:**
  - Expose `can_solve_2cut(raw_g: &Graph) -> Option<(i32, i32)>` using linear-time Tarjan articulation point discovery on $G \setminus \{u\}$.
  - Expose `solve_2cut_corridor(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>` which solves any 2-connected graph with a 2-vertex separator into Block A and Block B.
  - Zero checks on 4064 or graph name.

### 2. `macro_decomp/portfolio_788.rs`
- **Current State:**
  - Contains `solve_788(&g, timeout)` which enforces `raw_g.adjacency_list.len() == 4620`.
- **Target State:**
  - Expose `can_solve_alternating_pairs(raw_g: &Graph) -> bool`:
    - Checks that the graph has a high density of degree-2 chain pairs ($\ge |V| / 3$).
    - Validates that degree-2 contraction yields a 2-colorable alternating transition graph.
  - Expose `solve_alternating_pairs(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`.
  - Zero checks on 4620 or graph name.

### 3. `macro_decomp/bipartite.rs`
- **Current State:**
  - Contains `solve_746` with `super_hubs != vec![1430, 3641, 3735, 3790, 3960]` and hardcoded strip indices `[3, 8, 9, 13, 16]`.
  - Contains `solve_950` with hardcoded root sets `[164, 5787, ...]`.
- **Target State:**
  - Expose `detect_bipartite_super_hubs(raw_g: &Graph) -> Option<Vec<i32>>`:
    - Filters vertices with degree $\ge 500$.
    - Returns identified super-hubs dynamically without comparing to a constant ID vector.
  - Expose `solve_bipartite_macro(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`.

### 4. `pipeline/solver_pipeline.rs`
- **Current State:**
  ```rust
  let macro_tour_opt: Option<Vec<i32>> = match vertex_count {
      4286 => macro_bipartite::solve_746(&g, timeout_secs),
      4064 => macro_corridor::solve_710(&g, timeout_secs),
      4620 => macro_788::solve_788(&g, timeout_secs),
      6620 => macro_bipartite::solve_950(&g, timeout_secs),
      _ => None,
  };
  ```
- **Target State:**
  ```rust
  // Dynamic Topological Cascade:
  // 1. 2-Cut Articulation Separator
  if let Some((u, v)) = macro_corridor::can_solve_2cut(&g) {
      if let Some(tour) = macro_corridor::solve_2cut_corridor(&g, timeout_secs) {
          return verify_and_export(&g, &tour, start_time, output_tour_path);
      }
  }

  // 2. High-Degree Super-Hub Bipartite Decomposition
  if let Some(super_hubs) = macro_bipartite::detect_bipartite_super_hubs(&g) {
      if let Some(tour) = macro_bipartite::solve_bipartite_macro(&g, timeout_secs) {
          return verify_and_export(&g, &tour, start_time, output_tour_path);
      }
  }

  // 3. Degree-2 Alternating Pair Contraction
  if macro_788::can_solve_alternating_pairs(&g) {
      if let Some(tour) = macro_788::solve_alternating_pairs(&g, timeout_secs) {
          return verify_and_export(&g, &tour, start_time, output_tour_path);
      }
  }

  // 4. Fallback CEGAR
  fallback_cegar::solve_with_contraction(&g, remaining_timeout)
  ```
