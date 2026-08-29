# Global Supernode MTZ Potential Encoding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `GlobalSupernodeMTZ` into the base encoding pipeline at Round 0 in `hcp_solver.rs`, enforcing global topological ordering across $K \in [12, 20]$ supernodes to eliminate all macro-subcycles globally.

**Architecture:** Enhance `src/cegar-fix/src/metagraph_router.rs` and wire into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Wire `GlobalSupernodeMTZ` into `hcp_solver.rs` Round 0

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`, `src/cegar-fix/src/metagraph_router.rs`
- Test: `src/cegar-fix/tests/test_metagraph_router.rs`

- [ ] **Step 1: In `hcp_solver.rs`, activate `MetagraphRouter::encode_supernode_mtz` at Round 0 when graph has $\ge 50$ vertices with target $K=16$ supernodes ($K \in [4, 24]$)**
- [ ] **Step 2: Ensure `encode_supernode_mtz` is sound and leak-free**
- [ ] **Step 3: Run unit tests** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_metagraph_router`)
- [ ] **Step 4: Commit changes**

---

### Task 2: Integration & Full Workspace Tests

**Files:**
- Modify: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add integration test verifying global supernode MTZ order constraints in full CEGAR solver**
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
