# Parallel Randomized SAT Portfolio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `ParallelSatPortfolio` to run 3 diversified CaDiCaL worker instances concurrently across Cores 0, 1, 2, avoiding single-thread CDCL stagnation and exploiting heavy-tailed runtime speedups.

**Architecture:** New module `src/cegar-fix/src/parallel_sat_portfolio.rs`, integration into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL (`rustsat_cadical`, `rustsat`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: `ParallelSatPortfolio` Engine

**Files:**
- Create: `src/cegar-fix/src/parallel_sat_portfolio.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod parallel_sat_portfolio;`)
- Test: `src/cegar-fix/tests/test_parallel_sat_portfolio.rs`

**Interfaces:**
```rust
#[derive(Debug, Clone)]
pub enum PortfolioResult {
    Sat(rustsat::instances::BasicVarManager, Vec<rustsat::types::Lit>),
    Unsat,
    Interrupted,
}

pub struct ParallelSatPortfolio;

impl ParallelSatPortfolio {
    pub fn solve_portfolio(
        cnf: &rustsat::instances::Cnf,
        assumptions: &[rustsat::types::Lit],
        num_workers: usize, // default 3 (using cores 0, 1, 2)
    ) -> PortfolioResult;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_parallel_sat_portfolio.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_parallel_sat_portfolio`)
- [ ] **Step 3: Implement `ParallelSatPortfolio` in `src/cegar-fix/src/parallel_sat_portfolio.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_parallel_sat_portfolio`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `ParallelSatPortfolio` into CEGAR Loop in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Modify `hcp_solver.rs` to use `ParallelSatPortfolio::solve_portfolio` across CEGAR rounds**
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
