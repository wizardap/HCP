# Disjoint-Pocket Corridor LNS & Backbone Phase-Hinting Implementation Plan

- **Goal:** Eliminate the dispersed docking barrier on `graph868.col` by extracting two non-contiguous localized pockets on the Giant Cycle, freezing the intervening Macro-Arcs, and solving local 4-opt/6-opt cycle absorption with CaDiCaL, supplemented by global Backbone Phase-Hinting.
- **Spec:** [`docs/superpowers/specs/2026-09-12-disjoint-pocket-lns-design.md`](file:///home/ubuntu/HCP/docs/superpowers/specs/2026-09-12-disjoint-pocket-lns-design.md)
- **Scope:** 4 tasks, 5 files changed/created
- **Estimated time:** 45 minutes

---

## Global Constraints

- **Code Scope:** All Rust source code must reside in `src/cegar-fix/` and compile cleanly with `cargo build --release`.
- **CLI Flag Preservation:** Strictly preserve all existing CLI flags and options in `cegar-fix`.
- **Zero Tour Injection:** Never read, preload, or inspect `.tou` reference files.
- **Soundness & Zero Phantom Edges:** All tours and cycles must be 100% sound on the raw uncontracted graph $G$, verified against `g.adjacency_list`.
- **Bite-Sized TDD:** Every task must write failing tests first, verify RED, implement code, verify GREEN, and commit.

---

## File Structure

- `src/cegar-fix/src/disjoint_pocket_lns.rs`: Implements `DisjointPocketCandidate`, `DisjointPocketCorridor`, `find_pocket_candidates`, `build_disjoint_pockets`, `try_solve_disjoint_pockets`, and `repair`.
- `src/cegar-fix/src/lib.rs`: Exposes `pub mod disjoint_pocket_lns;`.
- `src/cegar-fix/src/hcp_solver.rs`: Wires `DisjointPocketLns::repair` and Backbone Phase-Hinting into the main CEGAR solver loop.
- `src/cegar-fix/tests/test_disjoint_pocket_candidates.rs`: Unit test validating dual pocket extraction and parity alignment.
- `src/cegar-fix/tests/test_disjoint_pocket_solving.rs`: Unit test validating 4-opt absorption across separated pockets.
- `src/cegar-fix/tests/test_disjoint_pocket_integration.rs`: Integration test verifying full solver pipeline with phase hinting.

---

## Tasks

### Task 1: Cài Đặt Cấu Trúc Dữ Liệu & Trích Xuất Dual Pocket (`disjoint_pocket_lns.rs`)

**Files:**
- Create: `src/cegar-fix/src/disjoint_pocket_lns.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_disjoint_pocket_candidates.rs`

- [ ] **Step 1: Write failing test in `src/cegar-fix/tests/test_disjoint_pocket_candidates.rs`**

```rust
use std::collections::{HashSet, HashMap};
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, Port, min_max};
use cegar_fix::disjoint_pocket_lns::DisjointPocketLns;

#[test]
fn test_find_disjoint_pocket_candidates() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // Setup 10 blocks: 0..9 on giant cycle, 10..11 on satellite cycle
    for b in 0..12 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);

    // Giant cycle: blocks 0..9 (ports 0..19)
    let mut giant = Vec::new();
    for b in 0..10 {
        giant.push(Port { block: b, end: 0 });
        giant.push(Port { block: b, end: 1 });
    }
    // Add external edges for giant: (1, 2), (3, 4), ..., (19, 0)
    for b in 0..10 {
        let u = port_to_node[&Port { block: b, end: 1 }];
        let v = port_to_node[&Port { block: (b + 1) % 10, end: 0 }];
        g.add_edge(u, v);
    }

    // Satellite cycle: blocks 10, 11
    let sat = vec![
        Port { block: 10, end: 0 }, Port { block: 10, end: 1 },
        Port { block: 11, end: 0 }, Port { block: 11, end: 1 },
    ];
    g.add_edge(port_to_node[&Port { block: 10, end: 1 }], port_to_node[&Port { block: 11, end: 0 }]);
    g.add_edge(port_to_node[&Port { block: 11, end: 1 }], port_to_node[&Port { block: 10, end: 0 }]);

    // Docking cross edges:
    // Block 10 connects to giant block 1 (pos 2..3)
    let u_sat10 = port_to_node[&Port { block: 10, end: 0 }];
    let v_g1 = port_to_node[&Port { block: 1, end: 1 }];
    g.add_edge(u_sat10, v_g1);

    // Block 11 connects to giant block 6 (pos 12..13)
    let u_sat11 = port_to_node[&Port { block: 11, end: 1 }];
    let v_g6 = port_to_node[&Port { block: 6, end: 0 }];
    g.add_edge(u_sat11, v_g6);

    let giant_pos = DisjointPocketLns::map_giant_coordinates(&giant, n_blocks);
    let candidates = DisjointPocketLns::find_pocket_candidates(
        &sat,
        &giant,
        &giant_pos,
        &g,
        &node_to_port,
        &port_to_node,
        1, // half_width B = 1 block
    );

    assert!(!candidates.is_empty(), "Must find at least 1 disjoint pocket candidate");
    let cand = &candidates[0];
    assert_ne!(cand.p1_center_block, cand.p2_center_block);
    assert!(cand.unfrozen_blocks.contains(&10));
    assert!(cand.unfrozen_blocks.contains(&11));

    let corridor = DisjointPocketLns::build_disjoint_pockets(cand, &sat, &giant);
    assert!(corridor.is_some(), "Corridor build must succeed");
    let corr = corridor.unwrap();
    assert_eq!(corr.pocket1_blocks.len(), 3);
    assert_eq!(corr.pocket2_blocks.len(), 3);
    assert!(corr.p1_entry_port != corr.p1_exit_port);
    assert!(corr.p2_entry_port != corr.p2_exit_port);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_disjoint_pocket_candidates`
Expected: Compile failure (module `disjoint_pocket_lns` not found).

- [ ] **Step 3: Implement `disjoint_pocket_lns.rs` and update `lib.rs`**

Implement:
- `DisjointPocketCandidate`
- `DisjointPocketCorridor`
- `map_giant_coordinates`
- `find_pocket_candidates`
- `build_disjoint_pockets`
Expose `pub mod disjoint_pocket_lns;` in `src/cegar-fix/src/lib.rs`.

- [ ] **Step 4: Run test to verify success**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_disjoint_pocket_candidates`
Expected: Pass (1 passed).

- [ ] **Step 5: Run full test suite**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Expected: All test targets pass.

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/disjoint_pocket_lns.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_disjoint_pocket_candidates.rs
git commit -m "feat(pocket): implement disjoint pocket candidate extraction and corridor builder"
```

---

### Task 2: Cài Đặt Macro-Arc Freezing & Local Multi-Pocket CEGAR Solving (`disjoint_pocket_lns.rs`)

**Files:**
- Modify: `src/cegar-fix/src/disjoint_pocket_lns.rs`
- Test: `src/cegar-fix/tests/test_disjoint_pocket_solving.rs`

- [ ] **Step 1: Write failing test in `src/cegar-fix/tests/test_disjoint_pocket_solving.rs`**

```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, Port, min_max};
use cegar_fix::disjoint_pocket_lns::DisjointPocketLns;
use cegar_fix::encoder::Encoder;

#[test]
fn test_disjoint_pocket_4opt_absorption() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 10 blocks: 0..7 giant cycle, 8..9 satellite cycle
    for b in 0..10 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle external edges: 0-1, 1-2, 2-3, 3-4, 4-5, 5-6, 6-7, 7-0
    // Port mappings: 2b+1 to 2(b+1)
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 8));
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 12));
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 0));

    // Satellite cycle: blocks 8, 9
    edges.insert(min_max(17, 18));
    edges.insert(min_max(19, 16));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Docking cross edges:
    // Block 8 connects to Block 1 (nodes 16 <-> 3, 17 <-> 2)
    g.add_edge(16, 3);
    g.add_edge(17, 2);
    // Block 9 connects to Block 5 (nodes 18 <-> 11, 19 <-> 10)
    g.add_edge(18, 11);
    g.add_edge(19, 10);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 2);

    let mut input_cycles = Vec::new();
    for pc in &cycs {
        input_cycles.push(pc.iter().map(|p| port_to_node[p]).collect());
    }

    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let repaired = DisjointPocketLns::repair(&input_cycles, &g, &contractor, &encoder, &base_cnf);
    assert_eq!(repaired.len(), 1, "Must absorb satellite cycle via 4-opt disjoint pockets");
    assert_eq!(repaired[0].len(), 20, "Must visit all 20 nodes");
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_disjoint_pocket_solving`
Expected: Compile failure (`repair` or methods not implemented).

- [ ] **Step 3: Implement `try_solve_disjoint_pockets` and `repair` in `disjoint_pocket_lns.rs`**

Implement:
- Freezing logic for Macro-Arc 1 and Macro-Arc 2 (unit assumptions for all external and internal edges outside $\Omega$).
- 4 boundary cuts asserted active:
  $x_{P_{1, in\_prev} \to P_{1, entry}}$, $x_{P_{1, exit} \to P_{1, exit\_next}}$,
  $x_{P_{2, in\_prev} \to P_{2, entry}}$, $x_{P_{2, exit} \to P_{2, exit\_next}}$.
- Degree-2 chain mutex for all blocks in $\Omega$.
- Local CEGAR loop: check cycles using `AlternatingPortEngine::get_cycles`. If target length $|C_0| + |C_{sat}|$ found, return `Some(merged)`. Else add local subcycle cut clauses.
- Full absorption loop in `repair`.

- [ ] **Step 4: Run test to verify success**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_disjoint_pocket_solving`
Expected: Pass (1 passed).

- [ ] **Step 5: Run full test suite and compile release**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: All tests pass, 0 compile errors.

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/disjoint_pocket_lns.rs src/cegar-fix/tests/test_disjoint_pocket_solving.rs
git commit -m "feat(pocket): implement Macro-Arc freezing and local multi-pocket CEGAR solving"
```

---

### Task 3: Đấu Nối Vào Solver Loop & Bổ Sung Backbone Phase-Hinting (`hcp_solver.rs`)

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_disjoint_pocket_integration.rs`

- [ ] **Step 1: Write integration test `src/cegar-fix/tests/test_disjoint_pocket_integration.rs`**

Verify that `DisjointPocketLns` is called in `solve_cegar` and all CLI flags continue to work.

- [ ] **Step 2: Update `src/cegar-fix/src/hcp_solver.rs`**

- Import `use crate::disjoint_pocket_lns::DisjointPocketLns;`.
- Wire `DisjointPocketLns::repair` right after `PortCorridorLns::repair`.
- Add Backbone Phase-Hinting:
  When `sol_cycles.len() > 1`:
  Identify the Giant Cycle (longest cycle, if length $\ge 80\%$ of vertices).
  For each directed edge $(u, v)$ in the Giant Cycle:
  If `encoder.graph_lit_map.contains_key(&(u, v))`, call `solver.phase(lit)` to bias CaDiCaL towards preserving the giant cycle backbone.

- [ ] **Step 3: Run integration test and full test suite**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_disjoint_pocket_integration`
Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Expected: 100% pass across all 58+ targets.

- [ ] **Step 4: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: 0 warnings, 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_disjoint_pocket_integration.rs
git commit -m "feat(solver): wire DisjointPocketLns and backbone phase-hinting into main CEGAR loop"
```

---

### Task 4: Chạy Benchmark Kiểm Chứng Độc Lập 1,800s Trên `graph868.col` (`taskset -c 0,1`)

**Files:**
- Execute: `target/release/cegar-fix` via `taskset -c 0,1`
- Output: `scratch/log_graph868_disjoint_pocket_1800s.txt`, `scratch/graph868_disjoint_pocket_found.tour`
- Verify: `scratch/verify_benchmarks.py`

- [ ] **Step 1: Copy release binary to `target/release/cegar-fix`**

```bash
mkdir -p target/release && cp src/cegar-fix/target/release/cegar-fix target/release/cegar-fix
```

- [ ] **Step 2: Launch benchmark with CPU pinning and timeout 1800s**

```bash
taskset -c 0,1 target/release/cegar-fix FHCPCS-col/graph868.col --timeout 1800 --alternating-engine --output-tour scratch/graph868_disjoint_pocket_found.tour 2>&1 | tee scratch/log_graph868_disjoint_pocket_1800s.txt
```

- [ ] **Step 3: Verify Hamiltonian tour if found**

If `scratch/graph868_disjoint_pocket_found.tour` is generated:
```bash
python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph868.col --tour scratch/graph868_disjoint_pocket_found.tour
```

- [ ] **Step 4: Update Progress Ledger & Report Telemetry**

Document telemetry and report results to user.
