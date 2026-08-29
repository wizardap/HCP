# Design Specification: Parallel Randomized SAT Portfolio (`ParallelSatPortfolio`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Motivation

### 1.1 Heavy-Tailed Runtime in CDCL Solving
SAT/CDCL solvers exhibit heavy-tailed runtime distributions on combinatorial HCP instances. A deterministic solver on a single thread can enter a pathological search branch taking $> 500\text{s}$, while an alternative decision/polarity sequence or random seed can find a satisfying assignment in $< 0.1\text{s}$.

### 1.2 Exploiting Assigned 3 Cores (Cores 0, 1, 2)
The environment allocates Cores 0, 1, 2 to the assistant (with Core 3 reserved for the user). `ParallelSatPortfolio` runs 3 diverse CaDiCaL instances concurrently across Cores 0, 1, 2:
1. **Worker 0 (Standard Base)**: Deterministic CaDiCaL with standard VSIDS variable selection and default phase saving.
2. **Worker 1 (Random Seed Diversified)**: CaDiCaL seeded with `seed = 42 + round * 17`, exploring alternative conflict-clause derivation paths.
3. **Worker 2 (Random Polarity Diversified)**: CaDiCaL seeded with `seed = 1337 + round * 31`, diversifying branching phase heuristics.

The first worker to find a solution instantly terminates the other workers and yields the model to the CEGAR loop.

---

## 2. Architectural Design & Interfaces

### 2.1 Structs & Signatures in `src/cegar-fix/src/parallel_sat_portfolio.rs`
```rust
use rustsat::instances::Cnf;
use rustsat::solvers::SolverResult;
use rustsat::types::Lit;

pub struct ParallelSatPortfolio;

#[derive(Debug, Clone)]
pub enum PortfolioResult {
    Sat(Vec<Lit>),
    Unsat,
    Interrupted,
}

impl ParallelSatPortfolio {
    /// Solves CNF across 3 parallel threads with distinct seeds/heuristics.
    /// Returns the first result found.
    pub fn solve_portfolio(
        cnf: &Cnf,
        assumptions: &[Lit],
        num_workers: usize, // default 3 (using cores 0, 1, 2)
    ) -> PortfolioResult;
}
```

### 2.2 Worker Diversity Strategies
- **Worker 0**: Standard CaDiCaL instance.
- **Worker 1**: CaDiCaL instance with randomized variable initial activity and seed.
- **Worker 2**: CaDiCaL instance with seed offset and alternative phase preferences.

### 2.3 Early Termination via Thread Channels
- A multi-producer single-consumer channel (`std::sync::mpsc::channel`) collects the first valid result.
- An `Arc<AtomicBool>` cancellation flag signals background workers to exit upon victory.

---

## 3. Integration into `hcp_solver.rs`
- In `solve_hamilton`, replace single-threaded `solver.solve()` / `solver.solve_assumps()` with `ParallelSatPortfolio::solve_portfolio`.
- If assumptions are provided, workers evaluate with assumptions first; if interrupted/unsat, workers fall back to unconstrained solving.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_parallel_sat_portfolio.rs`):**
   - Test `solve_portfolio` on simple SAT instances (3 workers finding SAT solution).
   - Test `solve_portfolio` on UNSAT instances (workers correctly returning UNSAT).
   - Test assumption-based solving and cancellation responsiveness.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR solver with `ParallelSatPortfolio` enabled.
3. **Benchmark Verification:**
   - Benchmark on `graph479.col` and `graph668.col` under `taskset -c 0,1,2 nice -n 19`.
