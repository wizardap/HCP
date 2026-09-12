# Cluster-Anchor Window Extraction & Unary MTZ Topological Ordering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Cluster-Anchor Window Selection and Unary MTZ Topological Ordering in `PortCorridorLns` to eliminate 2-block internal subcycle shattering and solve Hamiltonian subpaths inside corridors in < 10ms.

**Architecture:** 
1. `find_anchor_candidates` and `build_corridor_from_anchor` identify minimal-span dual-docking pairs on the Giant Cycle with complementary cross edges to satellite cycles, preserving the Two-Boundary Cut Invariant ($P_{entry}$ odd, $P_{exit}$ even, even port count).
2. `inject_unary_mtz_ordering` injects propositional ladder order variables $O_{b, t}$ into CaDiCaL, rendering isolated internal cycles unsatisfiable via BCP unit propagation.
3. The absorption engine iterates candidate anchors, solves the MTZ-constrained local CNF, and integrates the single unified path into the Giant Cycle.

**Tech Stack:** Rust (edition 2021), CaDiCaL FFI via `rustsat_cadical` (v0.1.3), `rustsat` (v0.6.6).

## Global Constraints

- All Rust code must reside in `src/cegar-fix/` and compile with 0 warnings/errors via `cargo build --release`.
- Strictly preserve all existing CLI flags and options in `cegar-fix` to prevent regressions across 1,001 benchmark instances.
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- All tours and cycles must be 100% sound on raw uncontracted graph G without phantom edges.
- Every modification must be validated through strict TDD (test first, RED -> GREEN).

---

### Task 1: Cài Đặt Cluster-Anchor Window Extraction (`AnchorCandidate`, `find_anchor_candidates`, `build_corridor_from_anchor`)

**Files:**
- Modify: `src/cegar-fix/src/port_corridor_lns.rs`
- Test: `src/cegar-fix/tests/test_port_corridor_cluster_anchor.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct AnchorCandidate {
      pub start_pos: usize,
      pub end_pos: usize,
      pub span: usize,
      pub sat_dock1: Port,
      pub sat_dock2: Port,
  }

  impl PortCorridorLns {
      pub fn find_anchor_candidates(
          sat: &[Port],
          giant: &[Port],
          giant_pos: &[usize],
          g: &Graph,
          node_to_port: &HashMap<i32, Port>,
          port_to_node: &HashMap<Port, i32>,
          max_span: usize,
      ) -> Vec<AnchorCandidate>;

      pub fn build_corridor_from_anchor(
          anchor: &AnchorCandidate,
          sat: &[Port],
          giant: &[Port],
          buffer: usize,
      ) -> Option<PortSubpathCorridor>;
  }
  ```

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_port_corridor_cluster_anchor.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_find_anchor_candidates_selects_closest_docking_pair() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 8 blocks: blocks 0..5 on giant, blocks 6, 7 on satellite
    for b in 0..8 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3, 4, 5 (12 ports)
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 8));
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 0));

    // Satellite cycle: blocks 6, 7 (4 ports)
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 12));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Docking cross edges:
    // Sat connects to block 1 (node 3) and block 2 (node 4) -> span = 2 ports (adjacent!)
    // Sat also connects to block 5 (node 11) -> span to block 1 is 8 ports (far!)
    g.add_edge(13, 3);
    g.add_edge(15, 4);
    g.add_edge(12, 11);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 2);

    let giant = &cycs[0];
    let sat = &cycs[1];
    let giant_pos = PortCorridorLns::map_giant_coordinates(giant, n_blocks);

    let anchors = PortCorridorLns::find_anchor_candidates(
        sat,
        giant,
        &giant_pos,
        &g,
        &node_to_port,
        &port_to_node,
        30,
    );

    assert!(!anchors.is_empty(), "Must find candidate anchors");
    // Shortest anchor must be between block 1 and block 2 (span <= 4 ports)
    assert!(anchors[0].span <= 4, "First anchor candidate must have minimal span <= 4, found {}", anchors[0].span);

    let corridor = PortCorridorLns::build_corridor_from_anchor(&anchors[0], sat, giant, 1);
    assert!(corridor.is_some(), "Corridor building must succeed");
    let c = corridor.unwrap();
    assert_eq!(c.sat_blocks.len(), 2);
    assert!(c.unfrozen_blocks.contains(&6) && c.unfrozen_blocks.contains(&7));
    assert!(c.unfrozen_blocks.contains(&1) && c.unfrozen_blocks.contains(&2));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_cluster_anchor`
Expected: FAIL (missing `AnchorCandidate`, `find_anchor_candidates`, `build_corridor_from_anchor`).

- [ ] **Step 3: Implement `AnchorCandidate`, `find_anchor_candidates`, and `build_corridor_from_anchor`**

In `src/cegar-fix/src/port_corridor_lns.rs`:
Add:
```rust
#[derive(Debug, Clone)]
pub struct AnchorCandidate {
    pub start_pos: usize,
    pub end_pos: usize,
    pub span: usize,
    pub sat_dock1: Port,
    pub sat_dock2: Port,
}

impl PortCorridorLns {
    pub fn find_anchor_candidates(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        max_span: usize,
    ) -> Vec<AnchorCandidate> {
        let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();
        let sat_ports_set: HashSet<Port> = sat.iter().copied().collect();
        let n_giant = giant.len();

        // Collect all (giant_port, sat_port) docking pairs
        let mut dock_pairs: Vec<(Port, Port)> = Vec::new();
        for &p_sat in sat {
            let u = port_to_node[&p_sat];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if let Some(&p_giant) = node_to_port.get(&v) {
                        if giant_blocks.contains(&p_giant.block) {
                            dock_pairs.push((p_giant, p_sat));
                        }
                    }
                }
            }
        }

        if dock_pairs.len() < 2 {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for i in 0..dock_pairs.len() {
            let (dg1, ds1) = dock_pairs[i];
            let pos1 = giant_pos[dg1.idx()];
            for j in (i + 1)..dock_pairs.len() {
                let (dg2, ds2) = dock_pairs[j];
                let pos2 = giant_pos[dg2.idx()];
                if ds1.block == ds2.block && ds1.end == ds2.end {
                    continue; // Must connect distinct satellite ports or blocks
                }

                let d_fwd = (pos2 + n_giant - pos1) % n_giant;
                let d_bwd = (pos1 + n_giant - pos2) % n_giant;
                let (start_pos, end_pos, span, p1, p2) = if d_fwd <= d_bwd {
                    (pos1, pos2, d_fwd, ds1, ds2)
                } else {
                    (pos2, pos1, d_bwd, ds2, ds1)
                };

                if span <= max_span * 2 {
                    candidates.push(AnchorCandidate {
                        start_pos,
                        end_pos,
                        span,
                        sat_dock1: p1,
                        sat_dock2: p2,
                    });
                }
            }
        }

        candidates.sort_by_key(|c| c.span);
        // Deduplicate overlapping positions
        candidates.dedup_by(|a, b| a.start_pos == b.start_pos && a.end_pos == b.end_pos);
        candidates
    }

    pub fn build_corridor_from_anchor(
        anchor: &AnchorCandidate,
        sat: &[Port],
        giant: &[Port],
        buffer: usize,
    ) -> Option<PortSubpathCorridor> {
        let n_giant = giant.len();
        let max_subpath_ports = if n_giant > 2 { n_giant - 2 } else { n_giant };

        let buf_ports = buffer * 2;
        let raw_start = (anchor.start_pos + n_giant - (buf_ports % n_giant)) % n_giant;
        let entry_idx = if raw_start % 2 == 0 {
            (raw_start + n_giant - 1) % n_giant
        } else {
            raw_start
        };

        let raw_end = (anchor.end_pos + buf_ports) % n_giant;
        let exit_idx = if raw_end % 2 != 0 {
            (raw_end + 1) % n_giant
        } else {
            raw_end
        };

        let mut total_subpath_ports = if exit_idx >= entry_idx {
            exit_idx - entry_idx + 1
        } else {
            (n_giant - entry_idx) + exit_idx + 1
        };

        if total_subpath_ports > max_subpath_ports {
            return None;
        }

        // Parity invariant: total_subpath_ports must be even
        if total_subpath_ports % 2 != 0 {
            total_subpath_ports += 1;
        }

        let entry_port = giant[entry_idx];
        let exit_port = giant[(entry_idx + total_subpath_ports - 1) % n_giant];

        let sat_blocks: HashSet<usize> = sat.iter().map(|p| p.block).collect();
        let mut giant_subpath_blocks = Vec::new();
        let mut unfrozen_blocks = sat_blocks.clone();

        for step in 0..total_subpath_ports {
            let idx = (entry_idx + step) % n_giant;
            let b = giant[idx].block;
            if unfrozen_blocks.insert(b) {
                giant_subpath_blocks.push(b);
            }
        }

        Some(PortSubpathCorridor {
            sat_blocks,
            giant_subpath_blocks,
            entry_port,
            exit_port,
            unfrozen_blocks,
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_cluster_anchor`
Expected: PASS (1/1 test passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/port_corridor_lns.rs src/cegar-fix/tests/test_port_corridor_cluster_anchor.rs
git commit -m "feat(corridor): implement cluster anchor candidate extraction and corridor builder"
```

---

### Task 2: Cài Đặt Unary MTZ Topological Ordering Generator (`inject_unary_mtz_ordering`)

**Files:**
- Modify: `src/cegar-fix/src/port_corridor_lns.rs`
- Test: `src/cegar-fix/tests/test_port_corridor_unary_mtz.rs`

**Interfaces:**
- Produces:
  ```rust
  impl PortCorridorLns {
      pub fn inject_unary_mtz_ordering(
          solver: &mut CaDiCaL,
          corridor: &PortSubpathCorridor,
          encoder: &Encoder,
          g: &Graph,
          node_to_port: &HashMap<i32, Port>,
          port_to_node: &HashMap<Port, i32>,
          next_free_var: &mut i32,
      ) -> (HashMap<(usize, usize), Lit>, usize);
  }
  ```

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_port_corridor_unary_mtz.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use cegar_fix::port_corridor_lns::PortCorridorLns;
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat_cadical::CaDiCaL;
use rustsat::types::TernaryVal;

#[test]
fn test_unary_mtz_forbids_disconnected_subcycles() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 4 blocks: 0, 1, 2, 3
    for b in 0..4 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    // Connect block 0 to 1, 1 to 2, 2 to 3, but ALSO allow a shortcut cycle between 1 and 2
    g.add_edge(1, 2); // 0 -> 1
    g.add_edge(3, 4); // 1 -> 2
    g.add_edge(5, 6); // 2 -> 3
    g.add_edge(3, 5); // 1 -> 2 cross
    g.add_edge(2, 4); // 1 -> 2 cross back (forms 2-cycle between block 1 & 2)

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let mut local_solver = CaDiCaL::default();
    for cl in base_cnf.iter() {
        let _ = local_solver.add_clause(cl.clone());
    }

    let mut sat_blocks = HashSet::new();
    sat_blocks.insert(1);
    sat_blocks.insert(2);
    let mut unfrozen_blocks = HashSet::new();
    for b in 0..4 { unfrozen_blocks.insert(b); }

    let corridor = cegar_fix::port_corridor_lns::PortSubpathCorridor {
        sat_blocks,
        giant_subpath_blocks: vec![0, 3],
        entry_port: Port { block: 0, end: 1 },
        exit_port: Port { block: 3, end: 0 },
        unfrozen_blocks,
    };

    let mut next_free_var = encoder.n_vars as i32 + 10;
    let (_order_vars, n_clauses) = PortCorridorLns::inject_unary_mtz_ordering(
        &mut local_solver,
        &corridor,
        &encoder,
        &g,
        &node_to_port,
        &port_to_node,
        &mut next_free_var,
    );

    assert!(n_clauses > 0, "Must inject MTZ clauses");

    let res = local_solver.solve();
    assert_eq!(res.unwrap(), SolverResult::Sat);
    let sol = local_solver.full_solution().unwrap();

    // Verify that the solution is a single 4-block path, NOT a shortcut + 2-block cycle!
    let mut active_edges = HashSet::new();
    for (&(u, v), &lit) in &encoder.graph_lit_map {
        if sol.lit_value(lit) == TernaryVal::True {
            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                if p1.block != p2.block {
                    active_edges.insert(min_max(u, v));
                }
            }
        }
    }

    let (cycs, _) = AlternatingPortEngine::get_cycles(&active_edges, &node_to_port, &port_to_node, n_blocks);
    // With MTZ, no disconnected cycles can exist inside the corridor!
    for c in &cycs {
        assert!(c.len() > 4, "No 2-block cycles permitted by MTZ");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_unary_mtz`
Expected: FAIL (missing `inject_unary_mtz_ordering`).

- [ ] **Step 3: Implement `inject_unary_mtz_ordering` in `port_corridor_lns.rs`**

In `src/cegar-fix/src/port_corridor_lns.rs`:
Implement:
1. Sort blocks in $\Omega$ with $b_{entry}$ at index 0 and $b_{exit}$ at index $K-1$.
2. Allocate $O_{b, t}$ variables for $t \in [1, K-1]$.
3. Add ladder implication clauses: $O_{b, t} \implies O_{b, t-1}$.
4. Add boundary clauses: $\neg O_{b_{entry}, 1}$, $O_{b_{exit}, K-1}$, and $\forall b \ne b_{exit}: \neg O_{b, K-1}$.
5. Add transition clauses for candidate external edges inside $\Omega$:
   - $x_{u \to v} \implies O_{b_2, 1}$ (for step 0).
   - $\forall t \in [1, K-2]: x_{u \to v} \land O_{b_1, t} \implies O_{b_2, t+1}$.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_unary_mtz`
Expected: PASS (1/1 test passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/port_corridor_lns.rs src/cegar-fix/tests/test_port_corridor_unary_mtz.rs
git commit -m "feat(corridor): implement unary MTZ topological ordering generator for local corridors"
```

---

### Task 3: Tích Hợp Động Cơ Hấp Thụ Đa Cụm & Thẩm Định Toàn Bộ Hệ Thống

**Files:**
- Modify: `src/cegar-fix/src/port_corridor_lns.rs`
- Test: `src/cegar-fix/tests/test_port_corridor_cluster_mtz_integration.rs`

**Interfaces:**
- Updates `try_absorb_single_cycle_with_others` and `repair` to:
  1. Extract candidate anchors via `find_anchor_candidates`.
  2. For each candidate anchor, build corridor and inject Unary MTZ.
  3. Solve local CaDiCaL with assumptions and decode Hamiltonian path.
  4. Return absorbed cycles when a satellite cycle is successfully merged.

- [ ] **Step 1: Write integration test**

Create `src/cegar-fix/tests/test_port_corridor_cluster_mtz_integration.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_corridor_cluster_mtz_merges_satellite_cycle() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 8 blocks total
    for b in 0..8 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3, 4, 5
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 8));
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 0));

    // Satellite cycle: blocks 6, 7
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 12));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Clustered docking cross edges connecting blocks 6, 7 into blocks 1, 2:
    // (13, 2), (14, 3), (15, 4), (12, 5)
    g.add_edge(13, 2);
    g.add_edge(14, 3);
    g.add_edge(15, 4);
    g.add_edge(12, 5);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 2);

    let mut input_cycles = Vec::new();
    for pc in &cycs {
        input_cycles.push(pc.iter().map(|p| port_to_node[p]).collect());
    }

    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let repaired = PortCorridorLns::repair(&input_cycles, &g, &contractor, &encoder, &base_cnf);
    assert_eq!(repaired.len(), 1, "Must absorb satellite cycle into 1 Hamiltonian cycle");
    assert_eq!(repaired[0].len(), 16, "Reconstructed cycle must visit all 16 nodes");
}
```

- [ ] **Step 2: Update `try_absorb_single_cycle_with_others` and `repair` in `port_corridor_lns.rs`**

Update `port_corridor_lns.rs` to iterate over candidate anchors from `find_anchor_candidates`, build corridor with `build_corridor_from_anchor`, inject Unary MTZ ordering with `inject_unary_mtz_ordering`, and return the unified cycle.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Expected: All 57+ test targets pass.

- [ ] **Step 4: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: 0 warnings, 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/port_corridor_lns.rs src/cegar-fix/tests/test_port_corridor_cluster_mtz_integration.rs
git commit -m "feat(corridor): wire cluster anchor search and unary MTZ ordering into absorption loop"
```

---

### Task 4: Chạy Benchmark Kiểm Chứng Độc Lập 1,800s Trên `graph868.col` (taskset -c 0,1)

**Files:**
- Execute: `target/release/cegar-fix` via `taskset -c 0,1`
- Output: `scratch/log_graph868_cluster_mtz_1800s.txt`, `scratch/graph868_cluster_mtz_found.tour`
- Verify: `scratch/verify_benchmarks.py`

- [ ] **Step 1: Copy release binary to `target/release/cegar-fix`**

```bash
mkdir -p target/release && cp src/cegar-fix/target/release/cegar-fix target/release/cegar-fix
```

- [ ] **Step 2: Execute benchmark with CPU pinning and timeout 1800s**

```bash
taskset -c 0,1 target/release/cegar-fix FHCPCS-col/graph868.col --timeout 1800 --alternating-engine --output-tour scratch/graph868_cluster_mtz_found.tour 2>&1 | tee scratch/log_graph868_cluster_mtz_1800s.txt
```

- [ ] **Step 3: Verify Hamiltonian tour if found**

If `scratch/graph868_cluster_mtz_found.tour` is generated:
```bash
python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph868.col --tour scratch/graph868_cluster_mtz_found.tour
```

- [ ] **Step 4: Update Progress Ledger & Report Telemetry**

Document telemetry and report results to user.
