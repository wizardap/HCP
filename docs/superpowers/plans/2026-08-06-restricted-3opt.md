# Restricted 3-Opt Subcycle Merging — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `--three-opt` CLI flag that enables a restricted 3-opt heuristic as a fallback inside the `two_opt()` loop in `cegar-ffi`, merging triplets of directed subcycles that 2-opt cannot handle.

**Architecture:** Three new functions (`merge_three_cycles`, `swap_three_nodes`, `cycle_join_three`) are added to `hcp_solver.rs`. The existing `two_opt()` loop is extended to call these functions when `merge_cycles()` fails and `three_opt == 1`. The `three_opt: i32` parameter is threaded down from CLI through `solve_hamilton → cegar → two_opt`.

**Tech Stack:** Rust, `clap` 3.x (existing), `cargo build` / `cargo check` for verification.

## Global Constraints

- All new code is in `src/cegar-ffi/` — do not create new crates or files outside this path.
- The graph is directed: edge `(u, v)` and edge `(v, u)` are distinct; always check `adjacency_list.get(&u).contains(&v)`.
- Do not break existing `opt` (`-t`) behavior — 3-opt is purely additive.
- Default value of `--three-opt` is `0` (disabled).
- Short flag for `--three-opt` is `-x`.
- After each task: `cargo check` in `src/cegar-ffi/` must pass with zero errors.

---

## File Structure

| File | Change |
|---|---|
| `src/cegar-ffi/src/options.rs` | Add `--three-opt` / `-x` argument to `clap` |
| `src/cegar-ffi/src/main.rs` | Read `three_opt` from matches, pass to `solve_hamilton` |
| `src/cegar-ffi/src/hcp_solver.rs` | Thread `three_opt` through signatures; add 3 new functions; extend `two_opt()` loop |

---

## Task 1: Add `--three-opt` CLI Flag

**Files:**
- Modify: `src/cegar-ffi/src/options.rs`
- Modify: `src/cegar-ffi/src/main.rs`

**Interfaces:**
- Produces: `three_opt: i32` variable in `main()`, value from CLI or default `0`.

- [ ] **Step 1: Add the argument to `options.rs`**

  In `src/cegar-ffi/src/options.rs`, inside the `App::new(...)` builder chain (after the existing `arcs-order` arg), add:

  ```rust
  .arg(
      Arg::with_name("three-opt")
          .short('x')
          .long("three-opt")
          .value_name("n")
          .help("Restricted 3-opt method:\n\
      0: Disabled (default)\n\
      1: Enabled — fallback inside 2-opt loop when 2-opt is stuck")
          .takes_value(true),
  )
  ```

- [ ] **Step 2: Read the value in `main.rs`**

  In `src/cegar-ffi/src/main.rs`, after the line:
  ```rust
  let arcs_order = matches.value_of_t::<i32>("arcs-order").unwrap_or(0);
  ```
  Add:
  ```rust
  let three_opt = matches.value_of_t::<i32>("three-opt").unwrap_or(0);
  ```

- [ ] **Step 3: Verify compilation**

  ```bash
  cd src/cegar-ffi && cargo check
  ```
  Expected: zero errors. The `three_opt` variable is unused at this point — Rust may emit a warning, which is acceptable.

- [ ] **Step 4: Commit**

  ```bash
  git add src/cegar-ffi/src/options.rs src/cegar-ffi/src/main.rs
  git commit -m "feat: add --three-opt CLI flag to cegar-ffi"
  ```

---

## Task 2: Thread `three_opt` Through Function Signatures

**Files:**
- Modify: `src/cegar-ffi/src/main.rs` — update `solve_hamilton` call
- Modify: `src/cegar-ffi/src/hcp_solver.rs` — update `solve_hamilton`, `cegar`, `two_opt` signatures

**Interfaces:**
- Consumes: `three_opt: i32` from Task 1.
- Produces:
  - `solve_hamilton(g, s, block_method, symmetry, opt, loop_prohibition, degree_order, arcs_order, three_opt, instant)`
  - `cegar(..., opt: i32, three_opt: i32, instant, previous_time)`
  - `two_opt(sol_cycles, solver, encoder, g, block_method, opt, three_opt) -> Vec<Vec<i32>>`

- [ ] **Step 1: Update `solve_hamilton` signature and body in `hcp_solver.rs`**

  Change the function signature from:
  ```rust
  pub fn solve_hamilton(g:Graph, _s:i32, block_method: i32,symmetry: i32 ,opt:i32,loop_prohibition: i32, degree_order:i32, arcs_order:i32, instant:Instant) {
  ```
  To:
  ```rust
  pub fn solve_hamilton(g:Graph, _s:i32, block_method: i32,symmetry: i32 ,opt:i32,loop_prohibition: i32, degree_order:i32, arcs_order:i32, three_opt:i32, instant:Instant) {
  ```

  Inside the body, update the `cegar(...)` call (currently line ~29) from:
  ```rust
  let (increment,block) = cegar(&mut encoder,solver,0,0, g,block_method, opt,instant,instant.elapsed());
  ```
  To:
  ```rust
  let (increment,block) = cegar(&mut encoder,solver,0,0, g,block_method, opt,three_opt,instant,instant.elapsed());
  ```

- [ ] **Step 2: Update `cegar` signature and body in `hcp_solver.rs`**

  Change the function signature from:
  ```rust
  fn cegar(encoder: &mut Encoder,solver:*mut Solver,mut count: i32,block_count:i32, g:Graph,block_method: i32, opt:i32, instant:Instant,previous_time:Duration) ->(i32,i32) {
  ```
  To:
  ```rust
  fn cegar(encoder: &mut Encoder,solver:*mut Solver,mut count: i32,block_count:i32, g:Graph,block_method: i32, opt:i32, three_opt:i32, instant:Instant,previous_time:Duration) ->(i32,i32) {
  ```

  Inside `cegar`, update the `two_opt(...)` call (currently line ~71) from:
  ```rust
  let cycles = two_opt(&sol_cycles,solver,encoder,&g,block_method,opt);
  ```
  To:
  ```rust
  let cycles = two_opt(&sol_cycles,solver,encoder,&g,block_method,opt,three_opt);
  ```

  And update the recursive `cegar(...)` call at the bottom of the function from:
  ```rust
  return cegar(encoder,solver, count,block_count, g, block_method,opt,instant,now);
  ```
  To:
  ```rust
  return cegar(encoder,solver, count,block_count, g, block_method,opt,three_opt,instant,now);
  ```

- [ ] **Step 3: Update `two_opt` signature in `hcp_solver.rs`**

  Change the function signature from:
  ```rust
  fn two_opt(sol_cycles:&Vec<Vec<i32>>,solver:*mut Solver,encoder: &mut Encoder,g:&Graph,block_method:i32,opt:i32) -> Vec<Vec<i32>>{
  ```
  To:
  ```rust
  fn two_opt(sol_cycles:&Vec<Vec<i32>>,solver:*mut Solver,encoder: &mut Encoder,g:&Graph,block_method:i32,opt:i32,three_opt:i32) -> Vec<Vec<i32>>{
  ```

  The body of `two_opt` does not yet use `three_opt` — that comes in Task 4.

- [ ] **Step 4: Update `solve_hamilton` call in `main.rs`**

  Change the call from:
  ```rust
  hcp_solver::solve_hamilton(g, solver, blocking, symmetry, two_opt, loop_prohibition,degree_order,arcs_order,instant);
  ```
  To:
  ```rust
  hcp_solver::solve_hamilton(g, solver, blocking, symmetry, two_opt, loop_prohibition,degree_order,arcs_order,three_opt,instant);
  ```

- [ ] **Step 5: Verify compilation**

  ```bash
  cd src/cegar-ffi && cargo check
  ```
  Expected: zero errors. `three_opt` will produce an unused-variable warning inside `two_opt` — acceptable for now.

- [ ] **Step 6: Commit**

  ```bash
  git add src/cegar-ffi/src/hcp_solver.rs src/cegar-ffi/src/main.rs
  git commit -m "refactor: thread three_opt parameter through solve_hamilton/cegar/two_opt"
  ```

---

## Task 3: Implement `swap_three_nodes` and `cycle_join_three`

**Files:**
- Modify: `src/cegar-ffi/src/hcp_solver.rs` — add two new functions after `cycle_join`

**Interfaces:**
- Consumes: `&Vec<i32>` (directed cycle as node sequence), `&Graph`
- Produces:
  - `swap_three_nodes(c1: &Vec<i32>, c2: &Vec<i32>, c3: &Vec<i32>, g: &Graph) -> Option<Vec<i32>>`
  - `cycle_join_three(c1: &Vec<i32>, c2: &Vec<i32>, c3: &Vec<i32>, i: usize, j: usize, k: usize, config: u8) -> Option<Vec<i32>>`

- [ ] **Step 1: Add `swap_three_nodes` to `hcp_solver.rs`**

  Add this function after the `cycle_join` function (after line ~309):

  ```rust
  /// Try to merge three directed cycles by a 3-edge swap.
  /// Tries two reconnection configurations for each (i, j, k) position.
  /// Config A: C1 -> C2 -> C3 -> C1  (u1->v2, u2->v3, u3->v1)
  /// Config B: C1 -> C3 -> C2 -> C1  (u1->v3, u3->v2, u2->v1)
  fn swap_three_nodes(c1: &Vec<i32>, c2: &Vec<i32>, c3: &Vec<i32>, g: &Graph) -> Option<Vec<i32>> {
      for i in 0..c1.len() {
          let u1 = c1[i];
          let v1 = c1[(i + 1) % c1.len()];
          let adjs_u1 = g.adjacency_list.get(&u1).unwrap();
          let adjs_v1 = g.adjacency_list.get(&v1).unwrap();

          for j in 0..c2.len() {
              let u2 = c2[j];
              let v2 = c2[(j + 1) % c2.len()];
              let adjs_u2 = g.adjacency_list.get(&u2).unwrap();
              let adjs_v2 = g.adjacency_list.get(&v2).unwrap();

              for k in 0..c3.len() {
                  let u3 = c3[k];
                  let v3 = c3[(k + 1) % c3.len()];
                  let adjs_u3 = g.adjacency_list.get(&u3).unwrap();

                  // Config A: u1->v2, u2->v3, u3->v1
                  if adjs_u1.contains(&v2) && adjs_u2.contains(&v3) && adjs_u3.contains(&v1) {
                      return cycle_join_three(c1, c2, c3, i, j, k, 0);
                  }
                  // Config B: u1->v3, u3->v2, u2->v1
                  if adjs_u1.contains(&v3) && adjs_u3.contains(&v2) && adjs_u2.contains(&v1) {
                      return cycle_join_three(c1, c2, c3, i, j, k, 1);
                  }
              }
          }
      }
      None
  }
  ```

- [ ] **Step 2: Add `cycle_join_three` to `hcp_solver.rs`**

  Add this function immediately after `swap_three_nodes`:

  ```rust
  /// Reconstruct a single merged cycle from three directed cycles given cut positions.
  /// config 0: C1[0..=i] + C2[j+1..] + C2[0..=j] ... (cyclic C1->C2->C3)
  /// config 1: C1[0..=i] + C3[k+1..] + C3[0..=k] ... (cyclic C1->C3->C2)
  fn cycle_join_three(
      c1: &Vec<i32>, c2: &Vec<i32>, c3: &Vec<i32>,
      i: usize, j: usize, k: usize,
      config: u8,
  ) -> Option<Vec<i32>> {
      let mut new_cycle = Vec::new();
      if config == 0 {
          // Route: c1[0..=i] -> c2 starting from j+1 -> c3 starting from k+1 -> back
          new_cycle.extend(&c1[0..=i]);
          new_cycle.extend(&c2[(j+1)%c2.len()..]);
          new_cycle.extend(&c2[..=(j)]);
          new_cycle.extend(&c3[(k+1)%c3.len()..]);
          new_cycle.extend(&c3[..=(k)]);
          if i + 1 < c1.len() {
              new_cycle.extend(&c1[i+1..]);
          }
      } else {
          // Config B: Route: c1[0..=i] -> c3 starting from k+1 -> c2 starting from j+1
          new_cycle.extend(&c1[0..=i]);
          new_cycle.extend(&c3[(k+1)%c3.len()..]);
          new_cycle.extend(&c3[..=(k)]);
          new_cycle.extend(&c2[(j+1)%c2.len()..]);
          new_cycle.extend(&c2[..=(j)]);
          if i + 1 < c1.len() {
              new_cycle.extend(&c1[i+1..]);
          }
      }
      Some(new_cycle)
  }
  ```

- [ ] **Step 3: Verify compilation**

  ```bash
  cd src/cegar-ffi && cargo check
  ```
  Expected: zero errors. Functions are defined but not yet called.

- [ ] **Step 4: Commit**

  ```bash
  git add src/cegar-ffi/src/hcp_solver.rs
  git commit -m "feat: add swap_three_nodes and cycle_join_three for 3-opt merging"
  ```

---

## Task 4: Implement `merge_three_cycles` and Wire Into `two_opt()` Loop

**Files:**
- Modify: `src/cegar-ffi/src/hcp_solver.rs` — add `merge_three_cycles`, extend `two_opt()` loop

**Interfaces:**
- Consumes: `swap_three_nodes` and `cycle_join_three` from Task 3; `three_opt: i32` from Task 2.
- Produces: When `three_opt == 1` and `merge_cycles()` fails with ≥ 3 active cycles, `merge_three_cycles()` is called as a fallback and its result is merged into the active cycle list if successful.

- [ ] **Step 1: Add `merge_three_cycles` function**

  Add this function after `merge_cycles` in `hcp_solver.rs`:

  ```rust
  /// Try to merge a triplet of active cycles using a 3-edge swap.
  /// Returns (merged, (idx1, idx2, idx3), new_cycle).
  fn merge_three_cycles(
      cycles: &Vec<Vec<i32>>,
      g: &Graph,
      active_cycles_number: &Vec<usize>,
  ) -> (bool, (usize, usize, usize), Vec<i32>) {
      let n = active_cycles_number.len();
      for a in 0..n {
          for b in a+1..n {
              for c in b+1..n {
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

- [ ] **Step 2: Extend the `two_opt()` loop to call `merge_three_cycles` as fallback**

  Inside the `two_opt()` function, the `while merged` loop body currently looks like:

  ```rust
  while merged {
      let (new_merged,merged_numbers,new_cycle) = merge_cycles(&cycles,g,&mut cache_vertex,&active_cycles_number,opt);
      merged = new_merged;
      
      if merged{
          cycles.push(new_cycle.clone());
          active_cycles_number.swap_remove(merged_numbers.1);
          active_cycles_number.swap_remove(merged_numbers.0);
          active_cycles_number.push(cycles.len()-1);
      }

      if active_cycles_number.len() == 1 || !merged{
          break
      }
      ...
  }
  ```

  Replace the entire `while merged { ... }` block with:

  ```rust
  while merged {
      let (new_merged, merged_numbers, new_cycle) = merge_cycles(&cycles, g, &mut cache_vertex, &active_cycles_number, opt);
      merged = new_merged;

      if merged {
          cycles.push(new_cycle.clone());
          active_cycles_number.swap_remove(merged_numbers.1);
          active_cycles_number.swap_remove(merged_numbers.0);
          active_cycles_number.push(cycles.len() - 1);
      }

      if active_cycles_number.len() == 1 {
          break;
      }

      // 3-opt fallback: when 2-opt is stuck and there are >= 3 cycles left
      if !merged && three_opt == 1 && active_cycles_number.len() >= 3 {
          let (three_merged, three_indices, three_cycle) = merge_three_cycles(&cycles, g, &active_cycles_number);
          if three_merged {
              cycles.push(three_cycle.clone());
              // Remove in reverse index order to avoid shifting issues
              let (ia, ib, ic) = three_indices;
              let mut remove_indices = [ia, ib, ic];
              remove_indices.sort_unstable_by(|a, b| b.cmp(a)); // descending
              for &idx in &remove_indices {
                  active_cycles_number.swap_remove(idx);
              }
              active_cycles_number.push(cycles.len() - 1);
              merged = true; // reset loop — re-try 2-opt on the new cycle set
              cache_vertex.clear(); // reset 2-opt cache for fresh attempt
              continue;
          }
      }

      if !merged {
          break;
      }

      if opt == 1 || opt == 4 {
          get_blocking_clauses(&vec!(new_cycle), solver, encoder, g, block_method);
      } else {
          maximam_cycles = new_cycle;
      }
  }
  ```

- [ ] **Step 3: Verify compilation**

  ```bash
  cd src/cegar-ffi && cargo check
  ```
  Expected: zero errors, zero new warnings beyond pre-existing ones.

- [ ] **Step 4: Do a quick smoke test**

  Build the binary and run on a small graph instance from `data/`:
  ```bash
  cd src/cegar-ffi && cargo build --release
  ./target/release/cegar-ffi -i ../../data/<any_small_graph_file> -t 1 --three-opt 1
  ```
  Expected: program runs to completion and prints either `s SATISFIABLE` or `s UNSATISFIABLE` without panic.

- [ ] **Step 5: Commit**

  ```bash
  git add src/cegar-ffi/src/hcp_solver.rs
  git commit -m "feat: implement merge_three_cycles and wire restricted 3-opt into two_opt loop"
  ```

---

## Task 5: Benchmark and Validate

**Files:**
- No code changes — run existing benchmarks only.

**Goal:** Confirm 3-opt reduces `incremented number` or total time on real instances vs baseline.

- [ ] **Step 1: Identify benchmark instances**

  List available graph files:
  ```bash
  ls data/
  ```
  Pick 2-3 instances that are non-trivial (e.g., not trivially 1-increment).

- [ ] **Step 2: Run baseline (2-opt only)**

  ```bash
  cd src/cegar-ffi
  ./target/release/cegar-ffi -i ../../data/<instance> -t 1 --three-opt 0 2>&1 | grep -E "incremented|overall time|merged"
  ```

- [ ] **Step 3: Run with 3-opt enabled**

  ```bash
  ./target/release/cegar-ffi -i ../../data/<instance> -t 1 --three-opt 1 2>&1 | grep -E "incremented|overall time|merged"
  ```

- [ ] **Step 4: Record results**

  Note the difference in `incremented number`, `number of merged cycles`, and `overall time` for each instance. If 3-opt shows no difference for a given instance, that is expected — it only activates when 2-opt gets stuck.

- [ ] **Step 5: Commit results note**

  ```bash
  git commit --allow-empty -m "test: benchmark restricted 3-opt vs baseline — results noted"
  ```
