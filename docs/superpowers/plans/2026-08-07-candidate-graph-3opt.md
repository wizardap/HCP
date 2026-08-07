# Candidate Graph Optimization for 3-Opt — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace brute-force $O(N^3)$ triplet enumeration in `merge_three_cycles` with a candidate-graph-filtered search that only examines cycle triplets connected by inter-cycle edges.

**Architecture:** Build `vertex_to_cycle` and `cycle_neighbors` data structures inside `merge_three_cycles`, then iterate only over triplets $(a, b, c)$ where $b \in \text{cycle\_neighbors}[a]$ and $c \in \text{cycle\_neighbors}[a] \cap \text{cycle\_neighbors}[b]$.

**Tech Stack:** Rust, `HashMap`, `HashSet` from `std::collections` (already imported in `hcp_solver.rs`).

## Global Constraints

- Only `merge_three_cycles` in `src/cegar-ffi/src/hcp_solver.rs` is modified. No other files change.
- No CLI changes. `--three-opt 1` automatically uses the optimized path.
- Must not regress results on `graph12.col`, `graph14.col`, `graph16.col`.

---

## Task 1: Rewrite `merge_three_cycles` with Candidate Graph Filtering

**Files:**
- Modify: `src/cegar-ffi/src/hcp_solver.rs` (function `merge_three_cycles`, lines ~275-296)

**Interfaces:**
- Consumes: `cycles: &Vec<Vec<i32>>`, `g: &Graph`, `active_cycles_number: &Vec<usize>`
- Produces: `(bool, (usize, usize, usize), Vec<i32>)` — same signature, same semantics, faster execution.

- [ ] **Step 1: Replace `merge_three_cycles` implementation**

  Replace the entire function body (lines ~275-296) with:

  ```rust
  /// Try to merge a triplet of active cycles using a 3-edge swap.
  /// Uses a candidate graph to only examine triplets with inter-cycle edges.
  /// Returns (merged, (idx1, idx2, idx3), new_cycle).
  fn merge_three_cycles(
      cycles: &Vec<Vec<i32>>,
      g: &Graph,
      active_cycles_number: &Vec<usize>,
  ) -> (bool, (usize, usize, usize), Vec<i32>) {
      let n = active_cycles_number.len();

      // Step 1: Build vertex -> active-index mapping
      let mut vertex_to_active: HashMap<i32, usize> = HashMap::new();
      for (active_idx, &cycle_idx) in active_cycles_number.iter().enumerate() {
          for &v in &cycles[cycle_idx] {
              vertex_to_active.insert(v, active_idx);
          }
      }

      // Step 2: Build cycle neighbor sets (candidate graph)
      let mut cycle_neighbors: Vec<HashSet<usize>> = vec![HashSet::new(); n];
      for (active_idx, &cycle_idx) in active_cycles_number.iter().enumerate() {
          for &u in &cycles[cycle_idx] {
              if let Some(adjs) = g.adjacency_list.get(&u) {
                  for &v in adjs {
                      if let Some(&neighbor_active) = vertex_to_active.get(&v) {
                          if neighbor_active != active_idx {
                              cycle_neighbors[active_idx].insert(neighbor_active);
                          }
                      }
                  }
              }
          }
      }

      // Step 3: Enumerate only connected triplets
      for a in 0..n {
          let neighbors_a: Vec<usize> = cycle_neighbors[a].iter()
              .filter(|&&b| b > a)
              .cloned()
              .collect();
          for &b in &neighbors_a {
              for &c in &cycle_neighbors[b] {
                  if c <= b { continue; }
                  if !cycle_neighbors[a].contains(&c) { continue; }
                  let ci = active_cycles_number[a];
                  let cj = active_cycles_number[b];
                  let ck = active_cycles_number[c];
                  if let Some(new_cycle) = swap_three_nodes(&cycles[ci], &cycles[cj], &cycles[ck], g) {
                      return (true, (a, b, c), new_cycle);
                  }
              }
          }
      }
      (false, (0, 0, 0), vec![])
  }
  ```

- [ ] **Step 2: Ensure `HashMap` is imported**

  Check that `HashMap` is already imported at the top of `hcp_solver.rs`. The existing line should be:
  ```rust
  use std::collections::{BTreeMap,HashSet};
  ```
  Update it to:
  ```rust
  use std::collections::{BTreeMap,HashMap,HashSet};
  ```

- [ ] **Step 3: Verify compilation**

  ```bash
  cd src/cegar-ffi && cargo check
  ```
  Expected: zero errors.

- [ ] **Step 4: Build release binary**

  ```bash
  cd src/cegar-ffi && cargo build --release
  ```

- [ ] **Step 5: Commit**

  ```bash
  git add src/cegar-ffi/src/hcp_solver.rs
  git commit -m "perf: replace brute-force 3-opt triplet search with candidate graph filtering"
  ```

---

## Task 2: Benchmark on graph470.col and Regression Check

**Files:** No code changes.

- [ ] **Step 1: Run graph470.col with 3-opt**

  ```bash
  cd src/cegar-ffi
  timeout 120 ./target/release/cegar-ffi -i ../../FHCPCS-col/graph470.col -t 1 -b 3 --three-opt 1 2>&1 | head -30
  ```
  Target: the local-search step (first increment) should complete within seconds, not minutes.

- [ ] **Step 2: Run regression check on small graphs**

  ```bash
  ./target/release/cegar-ffi -i ../../FHCPCS-col/graph12.col -t 1 -b 3 --three-opt 1 2>&1 | grep -E "connected|merged|incremented"
  ./target/release/cegar-ffi -i ../../FHCPCS-col/graph14.col -t 1 -b 3 --three-opt 1 2>&1 | grep -E "connected|merged|incremented"
  ```
  Expected: results should match or improve upon previous benchmarks.

- [ ] **Step 3: Commit benchmark note**

  ```bash
  git commit --allow-empty -m "test: benchmark candidate-graph 3-opt on graph470 and regression check"
  ```
