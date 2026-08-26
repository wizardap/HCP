# Component Meta-Graph & Guided Hub Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `ComponentMetaGraph` to detect cross-component edges $E(C_i, C_j)$, prune futile 2-opt pairs, and generate Multi-Component SEC cuts for the Two-Tier Hub Coordinator.

**Architecture:** A lightweight $O(|V| + |E|)$ meta-graph analyzes the $K$ subtours from `splice_macro_tour`. When $G_{\text{meta}}$ is disconnected, 2-opt is pruned and multi-component cut clauses are generated across meta-components to guide the SAT coordinator towards connected hub assignments.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Component Meta-Graph Engine

**Files:**
- Create: `src/cegar-fix/src/component_meta_graph.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_component_meta_graph.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct ComponentMetaGraph {
      pub num_components: usize,
      pub cross_edges: HashMap<(usize, usize), Vec<(i32, i32)>>,
      pub meta_adj: Vec<Vec<usize>>,
      pub meta_components: Vec<Vec<usize>>,
  }
  impl ComponentMetaGraph {
      pub fn build(cycles: &[Vec<i32>], g: &Graph) -> Self;
      pub fn has_merge_potential(&self, c1: usize, c2: usize) -> bool;
      pub fn is_connected(&self) -> bool;
      pub fn get_meta_components(&self) -> &[Vec<usize>];
  }
  ```

- [ ] **Step 1: Write the failing unit tests** in `src/cegar-fix/tests/test_component_meta_graph.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_component_meta_graph`)
- [ ] **Step 3: Implement `ComponentMetaGraph`** in `src/cegar-fix/src/component_meta_graph.rs` and export in `lib.rs`
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_component_meta_graph`)
- [ ] **Step 5: Commit changes**

---

### Task 2: 2-Opt Fast-Pruning Integration

**Files:**
- Modify: `src/cegar-fix/src/macro_splicer.rs:215-280`
- Test: `src/cegar-fix/tests/test_splicer.rs`

**Interfaces:**
- Consumes: `ComponentMetaGraph::build`, `has_merge_potential`
- Behavior: In `patch_cycles_2opt`, construct `ComponentMetaGraph` and skip any `(i, j)` pair where `!meta.has_merge_potential(i, j)`.

- [ ] **Step 1: Write test in `test_splicer.rs`** verifying fast-pruning on disconnected cycle pairs
- [ ] **Step 2: Run test to verify failure/baseline** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_splicer`)
- [ ] **Step 3: Integrate `ComponentMetaGraph` pruning in `patch_cycles_2opt`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_splicer`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Multi-Component SEC Cut Generation in Coordinator

**Files:**
- Modify: `src/cegar-fix/src/global_demand_coordinator.rs`
- Modify: `src/cegar-fix/src/two_tier_orchestrator.rs:180-220`
- Test: `src/cegar-fix/tests/test_coordinator.rs`

**Interfaces:**
- Consumes: `ComponentMetaGraph::get_meta_components`
- Produces: `add_multi_component_sec_cuts(&mut self, meta_components: &[Vec<usize>], cycles: &[Vec<i32>])`

- [ ] **Step 1: Write test in `test_coordinator.rs`** verifying multi-component SEC cuts
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_coordinator`)
- [ ] **Step 3: Implement multi-component SEC cuts in `GlobalDemandCoordinator` and wire into `TwoTierOrchestrator`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_coordinator`)
- [ ] **Step 5: Commit changes**

---

### Task 4: Benchmark Verification on `graph561.col`

**Files:**
- Verify: `FHCPCS-col/graph561.col`
- Command: `taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph561.col --auto 1`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph561.col` and verify multi-component SEC cuts in action**
- [ ] **Step 4: Commit benchmark report**
