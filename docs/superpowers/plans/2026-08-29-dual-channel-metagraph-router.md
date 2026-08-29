# Dual-Channel Metagraph Router Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement dual-channel decomposition and 2-channel supernode MTZ order encoding to solve global connectivity across two-port gadget networks without over-constraining 2-channel traversals.

**Architecture:** Enhancements to `src/cegar-fix/src/metagraph_router.rs` and integration into `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Dual-Channel Decomposition & MTZ Encoding in `MetagraphRouter`

**Files:**
- Modify: `src/cegar-fix/src/metagraph_router.rs`
- Test: `src/cegar-fix/tests/test_metagraph_router.rs`

**Interfaces:**
```rust
#[derive(Debug, Clone)]
pub struct ChannelModule {
    pub id: usize,
    pub parent_gadget_id: usize,
    pub channel_idx: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>,
}

impl MetagraphRouter {
    pub fn detect_dual_channels(g: &Graph) -> Vec<ChannelModule>;
    pub fn encode_dual_channel_mtz(
        channels: &[ChannelModule],
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    );
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_metagraph_router.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_metagraph_router`)
- [ ] **Step 3: Implement `detect_dual_channels` and `encode_dual_channel_mtz` in `src/cegar-fix/src/metagraph_router.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_metagraph_router`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire Dual-Channel Router into Base Encoding in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Modify `solve_hamilton` to invoke `detect_dual_channels` and `encode_dual_channel_mtz` at Round 0**
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
