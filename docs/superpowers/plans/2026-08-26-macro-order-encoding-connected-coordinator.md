# Macro Order-Encoding (MTZ) Connected Coordinator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Active Macro Order-Encoding (MTZ) in `GlobalDemandCoordinator` to eliminate disconnected macro subtours and guarantee single-cycle connectivity at the hub level.

**Architecture:** Embed Miller-Tucker-Zemlin unary ladder order variables ($O_{h, k} = [t_h \ge k]$) and directed macro transitions (Hub-Hub edges and Strip traversals) into CaDiCaL, mathematically preventing CaDiCaL from generating disconnected subtours.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Macro MTZ Encoder Engine

**Files:**
- Create: `src/cegar-fix/src/macro_mtz_encoder.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_macro_mtz.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct MacroMtzEncoder {
      pub root_hub: i32,
      pub hub_index: HashMap<i32, usize>,
      pub order_vars: HashMap<i32, Vec<Lit>>,
      pub dir_hh_vars: HashMap<(i32, i32), Lit>,
      pub dir_strip_vars: HashMap<(usize, i32, i32), Lit>,
  }
  impl MacroMtzEncoder {
      pub fn encode(
          solver: &mut CaDiCaL<'static, 'static>,
          next_var_id: &mut u32,
          decomp: &DecompositionResult,
          var_hh: &HashMap<(i32, i32), Lit>,
          var_d1: &HashMap<(usize, i32), Lit>,
      ) -> Self;
  }
  ```

- [ ] **Step 1: Write the failing unit tests** in `src/cegar-fix/tests/test_macro_mtz.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_macro_mtz`)
- [ ] **Step 3: Implement `MacroMtzEncoder`** in `src/cegar-fix/src/macro_mtz_encoder.rs` and export in `lib.rs`
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_macro_mtz`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Coordinator Integration with Macro MTZ

**Files:**
- Modify: `src/cegar-fix/src/global_demand_coordinator.rs`
- Test: `src/cegar-fix/tests/test_coordinator.rs`

**Interfaces:**
- Consumes: `MacroMtzEncoder::encode`
- Produces: `GlobalDemandCoordinator::new_with_mtz(g, decomp, enable_mtz)`

- [ ] **Step 1: Write test in `test_coordinator.rs`** verifying single-cycle guarantee with MTZ enabled
- [ ] **Step 2: Run test to verify failure/baseline** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_coordinator`)
- [ ] **Step 3: Integrate `MacroMtzEncoder` into `GlobalDemandCoordinator`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_coordinator`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Two-Tier Orchestrator Integration

**Files:**
- Modify: `src/cegar-fix/src/two_tier_orchestrator.rs`
- Test: `src/cegar-fix/tests/test_end_to_end.rs`

**Interfaces:**
- Consumes: `GlobalDemandCoordinator::new_with_mtz`
- Behavior: Enable MTZ in `solve_two_tier` when $N_H \le 200$.

- [ ] **Step 1: Write integration test in `test_end_to_end.rs`**
- [ ] **Step 2: Run test to verify failure/baseline** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_end_to_end`)
- [ ] **Step 3: Wire MTZ coordinator in `TwoTierOrchestrator`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_end_to_end`)
- [ ] **Step 5: Commit changes**

---

### Task 4: Benchmark Verification on `graph561.col`

**Files:**
- Verify: `FHCPCS-col/graph561.col`
- Command: `taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph561.col --auto 1`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph561.col` and verify single macro cycle generation**
- [ ] **Step 4: Document results and commit**
