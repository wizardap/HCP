# Extended Static Cycle & Gadget Perimeter Cutter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement static detection and clause injection for induced cycles of lengths $9 \dots 16$ (particularly 16-cycle gadget perimeters) at Round 0 to eliminate all $2^{60}$ gadget short-circuit traps.

**Architecture:** Enhance `src/cegar-fix/src/static_cycle_cutter.rs`, verify in `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: `StaticCycleCutter` Extension for Lengths 9..=16

**Files:**
- Modify: `src/cegar-fix/src/static_cycle_cutter.rs`
- Test: `src/cegar-fix/tests/test_static_cycle_cutter.rs`

**Interfaces:**
```rust
impl StaticCycleCutter {
    /// Extended static cycle detection for lengths 9..=16 (gadget perimeters)
    pub fn generate_static_small_cycle_cuts(
        g: &Graph,
        encoder: &Encoder,
    ) -> Cnf;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_static_cycle_cutter.rs` covering 10-, 12-, 14-, 16-cycles.
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 3: Implement extended 9..=16 cycle cutter in `src/cegar-fix/src/static_cycle_cutter.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire Extended Static Cuts into `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Verify Round 0 static cut injection logs and clauses**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
