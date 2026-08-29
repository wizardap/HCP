# Macro-Cycle Contraction & Exact SAT Cycle Stitcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `MacroCycleStitcher` to perform exact multi-cycle alternating symmetric difference merges on 2-factor subcycles using a lightweight subproblem, merging macro-cycles in $< 5\text{ms}$.

**Architecture:** New module `src/cegar-fix/src/macro_cycle_stitcher.rs`, integration into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: `MacroCycleStitcher` Engine

**Files:**
- Create: `src/cegar-fix/src/macro_cycle_stitcher.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod macro_cycle_stitcher;`)
- Test: `src/cegar-fix/tests/test_macro_cycle_stitcher.rs`

**Interfaces:**
```rust
pub struct MacroCycleStitcher;

impl MacroCycleStitcher {
    pub fn stitch_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Option<Vec<Vec<i32>>>;

    pub fn stitch_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_macro_cycle_stitcher.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_macro_cycle_stitcher`)
- [ ] **Step 3: Implement `MacroCycleStitcher` in `src/cegar-fix/src/macro_cycle_stitcher.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_macro_cycle_stitcher`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `MacroCycleStitcher` into CEGAR Loop in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Modify `hcp_solver.rs` to invoke `MacroCycleStitcher::stitch_until_fixed_point` in the CEGAR patching pipeline**
- [ ] **Step 2: Add integration test in `src/cegar-fix/tests/test_staged_solver.rs`**
- [ ] **Step 3: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 4: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
