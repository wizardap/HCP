# AlternatingPortEngine (Hybrid 2-Tier Alternating Cycle Engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `AlternatingPortEngine` (Hybrid 2-Tier Alternating Cycle Engine) in `src/cegar-fix` and wire it directly into the CEGAR loop of `hcp_solver.rs` to break bipartite symmetry traps and compress 2-factor subcycles down to Hamiltonian tours.

**Architecture:** A standalone module `src/cegar-fix/src/alternating_port_engine.rs` builds a bipartite port abstraction over degree-2 contracted blocks. Tier 1 executes 1-step greedy alternating cycle flips (<100ms) to compress disjoint cycles. Tier 2 executes bounded multi-hop alternating BFS escapes (depth 2..4, timeout <= 1.0s) when stalled at local minima. The engine is hooked directly into `hcp_solver.rs` right after CaDiCaL returns a 2-factor solution; if a single Hamiltonian tour is achieved, it immediately uncontracts and yields `SATISFIABLE`.

**Tech Stack:** Rust (edition 2021), `rustsat`, `rustsat-cadical`, standard library collections (`HashMap`, `HashSet`, `VecDeque`).

## Global Constraints

- Mọi mã nguồn Rust phải nằm trong `src/cegar-fix/` và biên dịch không lỗi với `cargo build --release`.
- Tuyệt đối bảo tồn các cờ dòng lệnh hiện hữu của `cegar-fix` để không gây hồi quy trên 1,001 bài toán benchmark.
- Zero Tour Injection: Tuyệt đối không đọc hoặc nạp trước file nghiệm chuẩn `.tou` trong toàn bộ pipeline giải.
- Mọi chu trình Hamilton tìm được phải được thẩm định độc lập 100% bằng logic `scratch/verify_benchmarks.py` trên đồ thị gốc chưa co cụm $G$.
- Các bước kiểm thử TDD phải viết test trước khi sửa đổi logic chính.

---

### Task 1: Xây Dựng Cấu Trúc Dữ Liệu Port & Động Cơ Tier 1 Greedy Alternating Flips

**Files:**
- Create: `src/cegar-fix/src/alternating_port_engine.rs`
- Modify: `src/cegar-fix/src/lib.rs:1-35`
- Test: `src/cegar-fix/tests/test_alternating_port_engine_tier1.rs`

**Interfaces:**
- Consumes: `Graph` from `crate::graph::Graph`, `Degree2Contractor` from `crate::contraction::Degree2Contractor`.
- Produces:
  - `pub struct Port { pub block: usize, pub end: u8 }`
  - `pub struct AlternatingPortEngine;`
  - `AlternatingPortEngine::repair_tier1(cycles: &[Vec<i32>], g: &Graph, contractor: &Degree2Contractor) -> Vec<Vec<i32>>`
  - Invariant: Degrees of all vertices remain strictly 2; cycles remain strictly valid in $G$.

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_alternating_port_engine_tier1.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, Port};

#[test]
fn test_port_creation_and_properties() {
    let p0 = Port { block: 0, end: 0 };
    let p1 = Port { block: 0, end: 1 };
    assert_ne!(p0, p1);
    assert_eq!(p0.block, p1.block);
}

#[test]
fn test_synthetic_bipartite_alternating_merge() {
    // Build a graph with 2 disjoint 4-cycles on 8 vertices (0..8)
    // contracted into 4 blocks of size 2.
    // Block 0: (0, 1), Block 1: (2, 3), Block 2: (4, 5), Block 3: (6, 7)
    // Cycle 1: 0 - 1 - 2 - 3 - 0
    // Cycle 2: 4 - 5 - 6 - 7 - 4
    // Cross edges available: (1, 4), (5, 2), (3, 6), (7, 0)
    let mut g = Graph::new();
    for v in 0..8 {
        g.add_vertex(v);
    }
    // Block internal edges
    g.add_edge(0, 1);
    g.add_edge(2, 3);
    g.add_edge(4, 5);
    g.add_edge(6, 7);

    // Cycle 1 external edges
    g.add_edge(1, 2);
    g.add_edge(3, 0);

    // Cycle 2 external edges
    g.add_edge(5, 6);
    g.add_edge(7, 4);

    // Cross inactive edges that allow alternating merge
    g.add_edge(1, 4);
    g.add_edge(5, 2);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);

    let initial_cycles = vec![
        vec![0, 1, 2, 3],
        vec![4, 5, 6, 7],
    ];

    let merged = AlternatingPortEngine::repair_tier1(&initial_cycles, &contracted_g, &contractor);
    assert_eq!(merged.len(), 1, "Tier 1 alternating flips should merge the 2 cycles into 1");
    assert_eq!(merged[0].len(), 8, "Merged cycle should contain all 8 vertices");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_alternating_port_engine_tier1`
Expected: FAIL with "cannot find module or type `alternating_port_engine`"

- [ ] **Step 3: Implement `Port` and `AlternatingPortEngine::repair_tier1`**

Create `src/cegar-fix/src/alternating_port_engine.rs`:
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

#[inline]
pub fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v { (u, v) } else { (v, u) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Port {
    pub block: usize,
    pub end: u8, // 0 for u, 1 for w
}

pub struct AlternatingPortEngine;

impl AlternatingPortEngine {
    pub fn get_cycles(
        edges: &HashSet<(i32, i32)>,
        node_to_port: &HashMap<i32, Port>,
        _port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
    ) -> (Vec<Vec<Port>>, HashMap<Port, Port>) {
        let mut pn: HashMap<Port, Port> = HashMap::with_capacity(n_blocks * 2);
        for &(u, v) in edges {
            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                pn.insert(p1, p2);
                pn.insert(p2, p1);
            }
        }

        let mut vis = HashSet::with_capacity(n_blocks * 2);
        let mut cycs = Vec::new();

        for b in 0..n_blocks {
            let p = Port { block: b, end: 0 };
            if !vis.contains(&p) && pn.contains_key(&p) {
                let mut c = Vec::new();
                let mut curr = p;
                while !vis.contains(&curr) {
                    vis.insert(curr);
                    c.push(curr);
                    if let Some(&nxt_p) = pn.get(&curr) {
                        vis.insert(nxt_p);
                        c.push(nxt_p);
                        curr = Port { block: nxt_p.block, end: 1 - nxt_p.end };
                    } else {
                        break;
                    }
                }
                cycs.push(c);
            }
        }
        cycs.sort_by_key(|c| std::cmp::Reverse(c.len()));
        (cycs, pn)
    }

    pub fn setup_ports(
        contractor: &Degree2Contractor,
    ) -> (usize, HashMap<i32, Port>, HashMap<Port, i32>) {
        let mut sorted_chains: Vec<(i32, i32)> = contractor.chain_map.keys().copied().collect();
        sorted_chains.sort();

        let mut pairs: Vec<(i32, i32)> = Vec::new();
        for (u, w) in sorted_chains {
            if u < w {
                pairs.push((u, w));
            }
        }

        let n_blocks = pairs.len();
        let mut node_to_port: HashMap<i32, Port> = HashMap::new();
        let mut port_to_node: HashMap<Port, i32> = HashMap::new();

        for (b_id, &(u, w)) in pairs.iter().enumerate() {
            let p_u = Port { block: b_id, end: 0 };
            let p_w = Port { block: b_id, end: 1 };
            node_to_port.insert(u, p_u);
            node_to_port.insert(w, p_w);
            port_to_node.insert(p_u, u);
            port_to_node.insert(p_w, w);
        }

        (n_blocks, node_to_port, port_to_node)
    }

    pub fn repair_tier1(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 || contractor.chain_map.is_empty() {
            return cycles.to_vec();
        }

        let (n_blocks, node_to_port, port_to_node) = Self::setup_ports(contractor);
        if n_blocks == 0 {
            return cycles.to_vec();
        }

        // Collect current 2-factor edges
        let mut current_edges: HashSet<(i32, i32)> = HashSet::new();
        let internal_edges: HashSet<(i32, i32)> = contractor.chain_map.keys().map(|&(u, v)| min_max(u, v)).collect();

        for c in cycles {
            let len = c.len();
            for i in 0..len {
                let e = min_max(c[i], c[(i + 1) % len]);
                if !internal_edges.contains(&e) {
                    current_edges.insert(e);
                }
            }
        }

        let (mut cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if cycs.is_empty() {
            return cycles.to_vec();
        }

        loop {
            let (cur_cycs, port_nbr) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
            if cur_cycs.len() <= 1 {
                break;
            }

            let mut port_to_cyc = HashMap::new();
            for (i, c) in cur_cycs.iter().enumerate() {
                for &p in c {
                    port_to_cyc.insert(p, i);
                }
            }

            let mut inactive_partner: HashMap<Port, Port> = HashMap::with_capacity(n_blocks * 2);
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    if let (Some(&raw_u), Some(&act_nbr)) = (port_to_node.get(&p), port_nbr.get(&p)) {
                        if let Some(nbrs) = g.adjacency_list.get(&raw_u) {
                            for &raw_v in nbrs {
                                if let Some(&inact_p) = node_to_port.get(&raw_v) {
                                    if inact_p != act_nbr {
                                        inactive_partner.insert(p, inact_p);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut vis_aux = HashSet::new();
            let mut alt_cycles = Vec::new();
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    if !vis_aux.contains(&p) && port_nbr.contains_key(&p) && inactive_partner.contains_key(&p) {
                        let mut ac = Vec::new();
                        let mut curr = p;
                        while !vis_aux.contains(&curr) && port_nbr.contains_key(&curr) {
                            vis_aux.insert(curr);
                            ac.push(curr);
                            let nxt1 = port_nbr[&curr];
                            vis_aux.insert(nxt1);
                            ac.push(nxt1);
                            if let Some(&nxt2) = inactive_partner.get(&nxt1) {
                                curr = nxt2;
                            } else {
                                break;
                            }
                        }
                        if ac.len() >= 4 {
                            alt_cycles.push(ac);
                        }
                    }
                }
            }

            let mut best_1step = None;
            let mut best_count = cur_cycs.len();
            let mut best_giant = cur_cycs[0].len();

            for ac in &alt_cycles {
                let mut touched = HashSet::new();
                for p in ac {
                    if let Some(&cid) = port_to_cyc.get(p) {
                        touched.insert(cid);
                    }
                }
                if touched.len() > 1 {
                    let mut rem = HashSet::new();
                    let mut add = HashSet::new();
                    for idx in (0..ac.len()).step_by(2) {
                        let p1 = ac[idx];
                        let p2 = ac[idx + 1];
                        rem.insert(min_max(port_to_node[&p1], port_to_node[&p2]));

                        let p3 = ac[(idx + 1) % ac.len()];
                        let p4 = ac[(idx + 2) % ac.len()];
                        add.insert(min_max(port_to_node[&p3], port_to_node[&p4]));
                    }
                    let mut cand = current_edges.clone();
                    for e in &rem { cand.remove(e); }
                    for e in add { cand.insert(e); }

                    let (cand_cycs, _) = Self::get_cycles(&cand, &node_to_port, &port_to_node, n_blocks);
                    if cand_cycs.len() < best_count || (cand_cycs.len() == best_count && cand_cycs[0].len() > best_giant) {
                        best_count = cand_cycs.len();
                        best_giant = cand_cycs[0].len();
                        best_1step = Some(cand);
                    }
                }
            }

            if let Some(improved) = best_1step {
                if best_count < cur_cycs.len() {
                    current_edges = improved;
                    continue;
                }
            }

            break;
        }

        // Convert resulting cycles back to Vec<Vec<i32>>
        let (final_port_cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        let mut final_cycles = Vec::new();
        for pc in final_port_cycs {
            let mut c = Vec::new();
            for p in pc {
                c.push(port_to_node[&p]);
            }
            final_cycles.push(c);
        }
        if final_cycles.is_empty() {
            cycles.to_vec()
        } else {
            final_cycles
        }
    }
}
```

Add `pub mod alternating_port_engine;` in `src/cegar-fix/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_alternating_port_engine_tier1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/alternating_port_engine.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_alternating_port_engine_tier1.rs
git commit -m "feat(alternating): implement port abstraction and Tier 1 greedy alternating engine"
```

---

### Task 2: Cài Đặt Tier 2 Bounded Multi-Hop BFS Escape & Thẩm Định Snapshot `graph868`

**Files:**
- Modify: `src/cegar-fix/src/alternating_port_engine.rs`
- Test: `src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs`

**Interfaces:**
- Consumes: `AlternatingPortEngine::repair_tier1`, `Port`.
- Produces:
  - `AlternatingPortEngine::repair_tier2_bounded_bfs(edges: &mut HashSet<(i32, i32)>, ...)`
  - `AlternatingPortEngine::repair(cycles: &[Vec<i32>], g: &Graph, contractor: &Degree2Contractor, max_depth: usize, timeout_ms: u64) -> Vec<Vec<i32>>`

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::file_operations;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};

#[test]
fn test_graph868_snapshot_reduction() {
    let graph_path = "../../FHCPCS-col/graph868.col";
    if !std::path::Path::new(graph_path).exists() {
        eprintln!("Skipping snapshot test: file not found");
        return;
    }
    let raw_g = file_operations::input_to_graph(graph_path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    let snapshot_path = "../../scratch/graph868_8_cycles.txt";
    if !std::path::Path::new(snapshot_path).exists() {
        eprintln!("Skipping snapshot test: 8-cycle checkpoint not found");
        return;
    }

    let content = std::fs::read_to_string(snapshot_path).unwrap();
    let mut initial_edges = HashSet::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            if let (Ok(u), Ok(v)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                initial_edges.insert(min_max(u, v));
            }
        }
    }

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&initial_edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 8, "Snapshot should initially have 8 cycles");

    // Convert to Vec<Vec<i32>> representation
    let mut input_cycles = Vec::new();
    for pc in &cycs {
        let mut c = Vec::new();
        for p in pc {
            c.push(port_to_node[p]);
        }
        input_cycles.push(c);
    }

    let t0 = std::time::Instant::now();
    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 4, 1000);
    let elapsed = t0.elapsed();

    assert!(repaired.len() <= 3, "Alternating engine must reduce 8 cycles down to <= 3 cycles, got {}", repaired.len());
    assert!(elapsed.as_millis() < 500, "Tier 1 + Tier 2 repair must finish within 500ms, took {:?}", elapsed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_alternating_port_engine_snapshot`
Expected: FAIL with "no function `repair` found on `AlternatingPortEngine`"

- [ ] **Step 3: Implement Tier 2 Bounded Multi-Hop BFS and main `repair` entry point**

Add to `src/cegar-fix/src/alternating_port_engine.rs`:
```rust
    pub fn repair(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        max_depth: usize,
        timeout_ms: u64,
    ) -> Vec<Vec<i32>> {
        let t_start = std::time::Instant::now();
        // 1. Run Tier 1 greedy flips
        let mut cur_cycles = Self::repair_tier1(cycles, g, contractor);
        if cur_cycles.len() <= 1 {
            return cur_cycles;
        }

        // 2. Setup ports for Tier 2
        let (n_blocks, node_to_port, port_to_node) = Self::setup_ports(contractor);
        if n_blocks == 0 {
            return cur_cycles;
        }

        let internal_edges: HashSet<(i32, i32)> = contractor.chain_map.keys().map(|&(u, v)| min_max(u, v)).collect();
        let mut current_edges: HashSet<(i32, i32)> = HashSet::new();
        for c in &cur_cycles {
            let len = c.len();
            for i in 0..len {
                let e = min_max(c[i], c[(i + 1) % len]);
                if !internal_edges.contains(&e) {
                    current_edges.insert(e);
                }
            }
        }

        // Build inactive adjacency list
        let mut inactive_adj: HashMap<Port, Vec<Port>> = HashMap::new();
        let (_, port_nbr) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);

        for b in 0..n_blocks {
            for end in 0..=1 {
                let p = Port { block: b, end };
                if let (Some(&raw_u), Some(&act_nbr)) = (port_to_node.get(&p), port_nbr.get(&p)) {
                    if let Some(nbrs) = g.adjacency_list.get(&raw_u) {
                        for &raw_v in nbrs {
                            if let Some(&inact_p) = node_to_port.get(&raw_v) {
                                if inact_p != act_nbr {
                                    inactive_adj.entry(p).or_default().push(inact_p);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Run bounded multi-hop BFS
        let (port_cycs, port_nbr) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if port_cycs.len() <= 1 {
            return cur_cycles;
        }

        for target_cycle_id in 1..port_cycs.len() {
            if t_start.elapsed().as_millis() as u64 >= timeout_ms {
                break;
            }
            let target_ports = &port_cycs[target_cycle_id];
            for &p0 in target_ports {
                let p_goal = port_nbr[&p0];
                use std::collections::VecDeque;
                let mut queue = VecDeque::new();
                let mut parent: HashMap<Port, (Port, Port)> = HashMap::new();
                let mut visited = HashSet::new();

                visited.insert(p0);
                queue.push_back((p0, 0));

                let mut found_flip = false;
                while let Some((curr, depth)) = queue.pop_front() {
                    if depth >= max_depth { continue; }
                    if let Some(inacts) = inactive_adj.get(&curr) {
                        for &inact in inacts {
                            let nxt2 = port_nbr[&inact];
                            if inact == p_goal && depth >= 1 {
                                let mut path = Vec::new();
                                let mut c_ptr = curr;
                                let mut i_ptr = inact;
                                path.push((c_ptr, i_ptr, p0));
                                while c_ptr != p0 {
                                    let (prev_curr, prev_inact) = parent[&c_ptr];
                                    path.push((prev_curr, prev_inact, c_ptr));
                                    c_ptr = prev_curr;
                                }
                                path.reverse();

                                let mut rem = HashSet::new();
                                let mut add = HashSet::new();
                                for &(c, i, n2) in &path {
                                    add.insert(min_max(port_to_node[&c], port_to_node[&i]));
                                    rem.insert(min_max(port_to_node[&i], port_to_node[&n2]));
                                }

                                let mut cand = current_edges.clone();
                                for e in &rem { cand.remove(e); }
                                for e in add { cand.insert(e); }

                                let (cand_cycs, _) = Self::get_cycles(&cand, &node_to_port, &port_to_node, n_blocks);
                                if cand_cycs.len() < port_cycs.len() {
                                    current_edges = cand;
                                    found_flip = true;
                                    break;
                                }
                            }

                            if !visited.contains(&nxt2) {
                                visited.insert(nxt2);
                                parent.insert(nxt2, (curr, inact));
                                queue.push_back((nxt2, depth + 1));
                            }
                        }
                    }
                    if found_flip { break; }
                }
                if found_flip { break; }
            }
        }

        // Final conversion
        let (final_port_cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        let mut res = Vec::new();
        for pc in final_port_cycs {
            let mut c = Vec::new();
            for p in pc {
                c.push(port_to_node[&p]);
            }
            res.push(c);
        }
        if res.is_empty() { cur_cycles } else { res }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_alternating_port_engine_snapshot`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/alternating_port_engine.rs src/cegar-fix/tests/test_alternating_port_engine_snapshot.rs
git commit -m "feat(alternating): implement Tier 2 multi-hop BFS and verify snapshot reduction"
```

---

### Task 3: Kết Nối Cờ CLI `--alternating-engine` Và Hook Vào Vòng Lặp CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/src/options.rs`
- Modify: `src/cegar-fix/src/hybrid_orchestrator.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_alternating_integration.rs`

**Interfaces:**
- Consumes: `AlternatingPortEngine::repair`, `HybridOptions::alternating_engine`.
- Produces:
  - CLI flag `--alternating-engine` / `--no-alternating-engine`
  - Immediate Hamiltonian tour return in `hcp_solver.rs` when `cycles.len() == 1`.
  - Zero regression on all existing unit and integration tests.

- [ ] **Step 1: Write the failing integration test**

Create `src/cegar-fix/tests/test_alternating_integration.rs`:
```rust
use cegar_fix::options::HybridOptions;
use cegar_fix::graph::Graph;
use cegar_fix::hcp_solver::solve_hamilton;

#[test]
fn test_alternating_cli_options_wiring() {
    let mut opts = HybridOptions::default();
    assert!(opts.alternating_engine, "alternating_engine should be true by default");

    opts.alternating_engine = false;
    assert!(!opts.alternating_engine);
}

#[test]
fn test_alternating_engine_solves_small_instance() {
    // 6-cycle graph
    let mut g = Graph::new();
    for v in 0..6 {
        g.add_vertex(v);
    }
    g.add_edge(0, 1);
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 0);

    let (iters, clauses, tour) = solve_hamilton(
        &g, 0, 0, false, 0, 0, 0, false, 0, false, false, 0, 0, 0, false, false, 0, true, 0, 0, 0, true
    );
    assert!(tour.is_some(), "Hamiltonian tour should be found");
    assert_eq!(tour.unwrap().len(), 6);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_alternating_integration`
Expected: FAIL with "no field `alternating_engine` on `HybridOptions`"

- [ ] **Step 3: Wire options, CLI parser, and `hcp_solver.rs` hook**

1. In `src/cegar-fix/src/options.rs`:
   Add `pub alternating_engine: bool` to `HybridOptions` (default `true`).
   Add `--alternating-engine` and `--no-alternating-engine` to CLI parser.

2. In `src/cegar-fix/src/hybrid_orchestrator.rs`:
   Propagate `alternating_engine: bool` in `HybridOptions`.

3. In `src/cegar-fix/src/hcp_solver.rs`:
   Add `alternating_engine: bool` parameter to `solve_hamilton` and `cegar`.
   Right after SAT extracts `sol_cycles`:
   ```rust
   let sol_cycles = if alternating_engine && !contractor.chain_map.is_empty() && sol_cycles.len() > 1 {
       let repaired = AlternatingPortEngine::repair(&sol_cycles, &g, contractor, 4, 1000);
       if repaired.len() < sol_cycles.len() {
           println!("AlternatingPortEngine: compressed subcycles from {} down to {} cycles", sol_cycles.len(), repaired.len());
       }
       if repaired.len() == 1 && repaired[0].len() == g.adjacency_list.len() {
           println!("*** 100% HAMILTONIAN TOUR FOUND VIA ALTERNATING PORT ENGINE! ***");
           let flat: Vec<i32> = repaired.into_iter().flatten().collect();
           let final_tour = contractor.uncontract_cycle(&flat);
           let line = final_tour.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
           let time = now - previous_time;
           let add_block_clauses_time = now - previous_time - sat_solving_time;
           println!("number of added block clauses = {}", clause_count);
           println!("add block clauses time = {:?}", add_block_clauses_time);
           println!("increment time = {:?}", time);
           println!();
           println!("solution: ");
           println!("{}\n", line);
           println!("s SATISFIABLE");
           return (count, clause_count, Some(final_tour));
       }
       repaired
   } else {
       sol_cycles
   };
   ```

4. Update all callers in `src/cegar-fix/src/main.rs`, `src/cegar-fix/src/hybrid_orchestrator.rs`, and test files.

- [ ] **Step 4: Run all tests to verify they pass**

Run: `cargo test`
Expected: 52/52 tests pass (100%)

- [ ] **Step 5: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: 0 errors

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/options.rs src/cegar-fix/src/hybrid_orchestrator.rs src/cegar-fix/src/main.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_alternating_integration.rs
git commit -m "feat(solver): wire AlternatingPortEngine CLI flag and CEGAR loop hook"
```
