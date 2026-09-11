# Bidirectional Alternating Port Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Bidirectional Alternating BFS (Meet-in-the-middle) with Smallest-Cycle-First prioritization in `AlternatingPortEngine` to expand the alternating cycle search horizon to 10–12 hops with $O(b^5)$ space, breaking multi-hop bipartite symmetry traps and accelerating subcycle reduction in `graph868.col`.

**Architecture:** 
- Traversal: Simultaneously expand forward from port $p_0$ through inactive-active alternating hops ($D_{fwd} \le 5$) and backward from port $p_{goal}$ through active-inactive hops ($D_{bwd} \le 5$).
- Collision: When frontiers intersect at port $p_{mid}$, reconstruct a simple, non-self-intersecting alternating cycle of up to 10–12 hops, verify 100% soundness against graph $G$, and apply the cycle-reducing flip followed by an immediate Tier 1 greedy sweep.
- Priority: Sort current subcycles in ascending order of length (smallest first) so small fragments ($k \le 16$) are targeted and merged into larger cycles first.

**Tech Stack:** Rust (2021 edition), CaDiCaL via rustsat-cadical, cargo test, taskset, Python 3 verification scripts.

## Global Constraints
- All Rust source code must reside in `src/cegar-fix/` and compile cleanly with `cargo build --release`.
- Strictly preserve all existing CLI flags and options in `cegar-fix`.
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- All tours must be 100% independently certified on raw uncontracted graph $G$ via `scratch/verify_benchmarks.py`.
- Follow strict TDD: Write failing unit/integration tests before implementing logic.

---

### Task 1: Cài Đặt Bidirectional Alternating BFS & Smallest-Cycle-First Heuristic

**Files:**
- Modify: `src/cegar-fix/src/alternating_port_engine.rs`
- Test: `src/cegar-fix/tests/test_alternating_bidirectional.rs`

**Interfaces:**
- Consumes: `Port`, `setup_ports`, `get_cycles`, `repair_tier1_edges`, `reconstruct_cycles` from `src/cegar-fix/src/alternating_port_engine.rs`.
- Produces:
  - `AlternatingPortEngine::repair_tier2_bidirectional_bfs(current_edges: &mut HashSet<(i32, i32)>, g: &Graph, node_to_port: &HashMap<i32, Port>, port_to_node: &HashMap<Port, i32>, n_blocks: usize, max_depth_per_dir: usize, timeout_ms: u64, t_start: std::time::Instant) -> bool`
  - Integrated `repair` utilizing bidirectional BFS with smallest-first cycle ordering.

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_alternating_bidirectional.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};

#[test]
fn test_bidirectional_bfs_finds_10hop_alternating_cycle() {
    // Build synthetic graph with 6 blocks (12 ports)
    // Connecting 2 separate cycles of 6 ports each via a 6-block corridor
    let mut g = Graph::new();
    let n_blocks = 6;
    let mut contractor = Degree2Contractor::new();

    for b in 0..n_blocks {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    // Active edges form 2 disjoint cycles:
    // Cycle 1: block 0, 1, 2 -> ports (0,1)-(2,0), (2,1)-(4,0), (4,1)-(0,0)
    // Cycle 2: block 3, 4, 5 -> ports (6,1)-(8,0), (8,1)-(10,0), (10,1)-(6,0)
    let mut initial_edges = HashSet::new();
    initial_edges.insert(min_max(1, 2));
    initial_edges.insert(min_max(3, 4));
    initial_edges.insert(min_max(5, 0));

    initial_edges.insert(min_max(7, 8));
    initial_edges.insert(min_max(9, 10));
    initial_edges.insert(min_max(11, 6));

    for &e in &initial_edges {
        g.add_edge(e.0, e.1);
    }

    // Add cross inactive edges forming a multi-hop alternating path between Cycle 1 and Cycle 2:
    // (1, 8) inactive, (3, 10) inactive
    g.add_edge(1, 8);
    g.add_edge(3, 10);

    let (blocks_count, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    assert_eq!(blocks_count, 6);

    let (cycs, _) = AlternatingPortEngine::get_cycles(&initial_edges, &node_to_port, &port_to_node, blocks_count);
    assert_eq!(cycs.len(), 2, "Initially must have 2 cycles");

    let mut input_cycles = Vec::new();
    for pc in &cycs {
        let mut c = Vec::new();
        for p in pc {
            c.push(port_to_node[p]);
        }
        input_cycles.push(c);
    }

    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 5, 1000);
    assert_eq!(repaired.len(), 1, "Bidirectional BFS must merge the 2 cycles into 1 Hamiltonian cycle");
}

#[test]
fn test_smallest_first_ordering() {
    let mut contractor = Degree2Contractor::new();
    let mut g = Graph::new();

    // 4 blocks: 2 for a small cycle, 2 for another
    for b in 0..4 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 0));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 4));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Add 2-opt cross edges (1, 6) and (3, 4)
    g.add_edge(1, 6);
    g.add_edge(3, 4);

    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &contractor.chain_map.keys().map(|&(u,_)| (u, Port{block: (u/2) as usize, end: (u%2) as usize})).collect(), &contractor.chain_map.keys().map(|&(u,_)| (Port{block: (u/2) as usize, end: (u%2) as usize}, u)).collect(), 4);

    let mut input_cycles = Vec::new();
    for c in &cycs {
        input_cycles.push(c.iter().map(|p| (p.block * 2 + p.end) as i32).collect());
    }

    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 4, 500);
    assert_eq!(repaired.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it compiles and fails or needs bidirectional implementation**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_alternating_bidirectional`
Expected: Compile failure or assertion failure if 10-hop cycle cannot be merged by unidirectional depth 4.

- [ ] **Step 3: Implement `repair_tier2_bidirectional_bfs` and update `repair` in `alternating_port_engine.rs`**

In `src/cegar-fix/src/alternating_port_engine.rs`:
1. Implement bidirectional search helper:
   ```rust
   pub fn repair_tier2_bidirectional_bfs(
       current_edges: &mut HashSet<(i32, i32)>,
       g: &Graph,
       node_to_port: &HashMap<i32, Port>,
       port_to_node: &HashMap<Port, i32>,
       n_blocks: usize,
       max_depth_per_dir: usize,
       timeout_ms: u64,
       t_start: std::time::Instant,
   ) -> bool
   ```
2. Sort `cur_cycs` by length ascending (Smallest-Cycle-First).
3. For each small cycle $C_i$, for each port $p_0 \in C_i$, define $p_{goal} = \text{port\_nbr}[p_0]$:
   - Expand forward frontier $\mathcal{F}_{fwd}$ from $p_0$ (alternating inactive $\to$ active) up to `max_depth_per_dir`.
   - Expand backward frontier $\mathcal{F}_{bwd}$ from $p_{goal}$ (alternating active $\to$ inactive) up to `max_depth_per_dir`.
   - On collision at $p_{mid}$:
     - Splicing paths: forward path from $p_0$ to $p_{mid}$, backward path from $p_{goal}$ to $p_{mid}$.
     - Check disjointness: all traversed ports are distinct except $p_{mid}$.
     - Build `add` and `rem` edge sets.
     - Validate all `add` edges exist in $G$ and all `rem` edges exist in `current_edges`.
     - Test if candidate edges reduce cycle count. If so, update `current_edges` and return `true`.
4. Call `repair_tier1_edges` immediately whenever an improvement is found.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_alternating_bidirectional`
Expected: PASS (2/2 tests pass).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/alternating_port_engine.rs src/cegar-fix/tests/test_alternating_bidirectional.rs
git commit -m "feat(alternating): implement Bidirectional Alternating BFS with smallest-first cycle ordering"
```

---

### Task 2: Thẩm Định Snapshot `graph868` & Đấu Nối Cấu Hình Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:555-585`
- Test: `src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs`

**Interfaces:**
- Consumes: `AlternatingPortEngine::repair` with updated bidirectional depth.
- Produces:
  - Robust cycle reduction on `scratch/graph868_giant_1686_full_edges.txt`.
  - Zero regression on all existing tests (`cargo test --tests`).

- [ ] **Step 1: Update snapshot test to verify bidirectional depth performance**

In `src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs`:
Ensure `AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 5, 1000)` reduces snapshot cycles and runs in $< 200$ms.

- [ ] **Step 2: Run snapshot test**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_alternating_port_engine_snapshot`
Expected: PASS.

- [ ] **Step 3: Update `hcp_solver.rs` call site**

In `src/cegar-fix/src/hcp_solver.rs` line 558:
Set `max_depth = 5` and `timeout_ms = 2000` for `AlternatingPortEngine::repair`:
```rust
let repaired = AlternatingPortEngine::repair(&sol_cycles, &g, contractor, 5, 2000);
```

- [ ] **Step 4: Run full test suite**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Expected: 54/54 tests pass (100%).

- [ ] **Step 5: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs
git commit -m "feat(solver): tune AlternatingPortEngine depth and verify snapshot cycle reduction"
```

---

### Task 3: Chạy Benchmark Kiểm Chứng Độc Lập 1,800s Trên `graph868.col`

**Files:**
- Execute: `target/release/cegar-fix` via `taskset -c 0,1`
- Verify: `scratch/verify_benchmarks.py`
- Output: `scratch/log_graph868_bidirectional_1800s.txt`, `scratch/graph868_bidirectional_found.tour`

- [ ] **Step 1: Copy release binary to `target/release/cegar-fix`**

```bash
mkdir -p target/release && cp src/cegar-fix/target/release/cegar-fix target/release/cegar-fix
```

- [ ] **Step 2: Run benchmark with CPU pinning and timeout 1800s**

```bash
taskset -c 0,1 target/release/cegar-fix FHCPCS-col/graph868.col --timeout 1800 --alternating-engine --output-tour scratch/graph868_bidirectional_found.tour 2>&1 | tee scratch/log_graph868_bidirectional_1800s.txt
```

- [ ] **Step 3: Verify Hamiltonian tour if found**

If `scratch/graph868_bidirectional_found.tour` is produced:
```bash
python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph868.col --tour scratch/graph868_bidirectional_found.tour
```

- [ ] **Step 4: Document telemetry and report to user**

Update progress ledger and compile comprehensive telemetry table comparing unidirectional vs bidirectional BFS performance.
