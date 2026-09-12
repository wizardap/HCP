# Port-Corridor LNS Subcycle Absorption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `PortCorridorLns`, a focused Single-Cycle Large Neighborhood Search (LNS) engine utilizing Connected Port-Subpath Corridors with strict two-boundary cut invariants, to sequentially absorb the remaining 32 satellite cycles of `graph868.col` into the Giant Cycle without triggering parity UNSAT barriers.

**Architecture:** 
- Port-Coordinate Mapping: Map the Giant Cycle into a circular port coordinate space $P_0, \dots, P_{2L-1}$.
- Focused Corridor Selection: For each satellite cycle $C_i$ (Smallest-Cycle-First), find its docking ports on $C_{giant}$, determine the minimal circular span with buffer $\Delta = 2$ blocks, and unfreeze only $C_i$ and the contiguous giant subpath.
- Two-Boundary Parity Soundness: Constrain the corridor boundary to exactly 2 cut edges ($P_{entry}$ and $P_{exit}$), freezing all external edges. Solve localized Hamiltonian path routing via incremental CaDiCaL with subcycle CEGAR learning inside $\Omega$ in $< 5$ms per cycle.

**Tech Stack:** Rust (2021 edition), CaDiCaL via `rustsat-cadical`, cargo test, Python 3 verification scripts.

## Global Constraints
- All Rust source code must reside in `src/cegar-fix/` and compile cleanly with `cargo build --release`.
- Strictly preserve all existing CLI flags and options in `cegar-fix`.
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- All tours must be 100% independently certified on raw uncontracted graph $G$ via `scratch/verify_benchmarks.py`.
- Follow strict TDD: Write failing unit/integration tests before implementing logic.

---

### Task 1: Cài Đặt Port-Coordinate Mapping & Corridor Extraction (`port_corridor_lns.rs`)

**Files:**
- Create: `src/cegar-fix/src/port_corridor_lns.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_port_corridor_extraction.rs`

**Interfaces:**
- Consumes: `Port`, `min_max`, `setup_ports`, `get_cycles` from `crate::alternating_port_engine`.
- Produces:
  - `pub struct PortSubpathCorridor { pub sat_blocks: HashSet<usize>, pub giant_subpath_blocks: Vec<usize>, pub entry_port: Port, pub exit_port: Port, pub unfrozen_blocks: HashSet<usize> }`
  - `PortCorridorLns::map_giant_coordinates(giant: &[Port], n_blocks: usize) -> Vec<usize>`
  - `PortCorridorLns::find_docking_ports(sat: &[Port], giant_blocks: &HashSet<usize>, g: &Graph, node_to_port: &HashMap<i32, Port>, port_to_node: &HashMap<Port, i32>) -> Vec<Port>`
  - `PortCorridorLns::build_subpath_corridor(sat: &[Port], giant: &[Port], giant_pos: &[usize], g: &Graph, node_to_port: &HashMap<i32, Port>, port_to_node: &HashMap<Port, i32>, max_span: usize, buffer: usize) -> Option<PortSubpathCorridor>`

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_port_corridor_extraction.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_coordinate_mapping_and_corridor_extraction() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 6 blocks (12 ports)
    // Giant cycle: blocks 0, 1, 2, 3 (8 ports)
    // Satellite cycle: blocks 4, 5 (4 ports)
    for b in 0..6 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant: (1, 2), (3, 4), (5, 6), (7, 0)
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 0));

    // Satellite: (9, 10), (11, 8)
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 8));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Inactive docking edges from satellite to giant:
    // (9, 2) and (11, 4)
    g.add_edge(9, 2);
    g.add_edge(11, 4);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);

    assert_eq!(cycs.len(), 2);
    let giant = &cycs[0];
    let sat = &cycs[1];

    let giant_pos = PortCorridorLns::map_giant_coordinates(giant, n_blocks);
    let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();

    let docking = PortCorridorLns::find_docking_ports(sat, &giant_blocks, &g, &node_to_port, &port_to_node);
    assert_eq!(docking.len(), 2, "Must identify 2 docking ports in giant cycle");

    let corridor = PortCorridorLns::build_subpath_corridor(sat, giant, &giant_pos, &g, &node_to_port, &port_to_node, 10, 1)
        .expect("Must successfully construct corridor");

    assert!(corridor.unfrozen_blocks.contains(&4));
    assert!(corridor.unfrozen_blocks.contains(&5));
    assert!(corridor.unfrozen_blocks.len() >= 4, "Must unfreeze satellite blocks + giant subpath blocks");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_extraction`
Expected: Compile failure (module `port_corridor_lns` does not exist).

- [ ] **Step 3: Implement `port_corridor_lns.rs` foundation and update `lib.rs`**

Create `src/cegar-fix/src/port_corridor_lns.rs`:
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::alternating_port_engine::Port;

#[derive(Debug, Clone)]
pub struct PortSubpathCorridor {
    pub sat_blocks: HashSet<usize>,
    pub giant_subpath_blocks: Vec<usize>,
    pub entry_port: Port,
    pub exit_port: Port,
    pub unfrozen_blocks: HashSet<usize>,
}

pub struct PortCorridorLns;

impl PortCorridorLns {
    pub fn map_giant_coordinates(giant: &[Port], n_blocks: usize) -> Vec<usize> {
        let mut pos = vec![usize::MAX; n_blocks * 2];
        for (i, &p) in giant.iter().enumerate() {
            pos[p.idx()] = i;
        }
        pos
    }

    pub fn find_docking_ports(
        sat: &[Port],
        giant_blocks: &HashSet<usize>,
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
    ) -> Vec<Port> {
        let mut docking = HashSet::new();
        for &p in sat {
            let u = port_to_node[&p];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if let Some(&p_v) = node_to_port.get(&v) {
                        if giant_blocks.contains(&p_v.block) {
                            docking.insert(p_v);
                        }
                    }
                }
            }
        }
        docking.into_iter().collect()
    }

    pub fn build_subpath_corridor(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        max_span: usize,
        buffer: usize,
    ) -> Option<PortSubpathCorridor> {
        let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();
        let docking = Self::find_docking_ports(sat, &giant_blocks, g, node_to_port, port_to_node);
        if docking.is_empty() {
            return None;
        }

        let n_giant_ports = giant.len();
        let mut dock_positions: Vec<usize> = docking.iter().map(|p| giant_pos[p.idx()]).collect();
        dock_positions.sort();

        // Find the shortest circular interval covering all docking positions (or densest cluster within max_span)
        let mut best_start = dock_positions[0];
        let mut best_len = n_giant_ports;

        for i in 0..dock_positions.len() {
            let start = dock_positions[i];
            let end = dock_positions[(i + dock_positions.len() - 1) % dock_positions.len()];
            let len = if end >= start {
                end - start + 1
            } else {
                (n_giant_ports - start) + end + 1
            };
            if len < best_len {
                best_len = len;
                best_start = start;
            }
        }

        // If best_len in blocks exceeds max_span * 2, clamp to max_span * 2
        let span_len = std::cmp::min(best_len, max_span * 2);

        // Add buffer
        let buf_ports = buffer * 2;
        let entry_idx = (best_start + n_giant_ports - buf_ports) % n_giant_ports;
        let total_subpath_ports = span_len + buf_ports * 2;
        let exit_idx = (entry_idx + total_subpath_ports - 1) % n_giant_ports;

        let entry_port = giant[entry_idx];
        let exit_port = giant[exit_idx];

        let sat_blocks: HashSet<usize> = sat.iter().map(|p| p.block).collect();
        let mut giant_subpath_blocks = Vec::new();
        let mut unfrozen_blocks = sat_blocks.clone();

        for step in 0..total_subpath_ports {
            let idx = (entry_idx + step) % n_giant_ports;
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

In `src/cegar-fix/src/lib.rs`:
Add:
```rust
pub mod port_corridor_lns;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_extraction`
Expected: PASS (1/1 test passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/port_corridor_lns.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_port_corridor_extraction.rs
git commit -m "feat(corridor): implement port coordinate mapping and subpath corridor builder"
```

---

### Task 2: Cài Đặt SAT Assumption Generator, Localized Subcycle CEGAR & Single-Cycle Absorption

**Files:**
- Modify: `src/cegar-fix/src/port_corridor_lns.rs`
- Test: `src/cegar-fix/tests/test_port_corridor_single_absorb.rs`

**Interfaces:**
- Consumes: `PortSubpathCorridor`, `Encoder`, `Cnf`, `CaDiCaL` from `rustsat_cadical`.
- Produces:
  - `PortCorridorLns::try_absorb_single_cycle(sat: &[Port], giant: &[Port], giant_pos: &[usize], g: &Graph, contractor: &Degree2Contractor, node_to_port: &HashMap<i32, Port>, port_to_node: &HashMap<Port, i32>, encoder: &Encoder, base_cnf: &Cnf) -> Option<Vec<Port>>`
  - `PortCorridorLns::repair(cycles: &[Vec<i32>], g: &Graph, contractor: &Degree2Contractor, encoder: &Encoder, base_cnf: &Cnf) -> Vec<Vec<i32>>`

- [ ] **Step 1: Write the failing test**

Create `src/cegar-fix/tests/test_port_corridor_single_absorb.rs`:
```rust
use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_corridor_absorbs_satellite_cycle() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 6 blocks
    for b in 0..6 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 0));

    // Satellite cycle: blocks 4, 5
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 8));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Docking cross edges: (9, 2), (11, 4)
    g.add_edge(9, 2);
    g.add_edge(11, 4);

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
    assert_eq!(repaired.len(), 1, "PortCorridorLns must absorb satellite cycle into 1 Hamiltonian cycle");
    assert_eq!(repaired[0].len(), 12, "Reconstructed cycle must visit all 12 nodes");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_single_absorb`
Expected: Compile failure (`repair` method not yet implemented).

- [ ] **Step 3: Implement `try_absorb_single_cycle` and `repair` in `port_corridor_lns.rs`**

In `src/cegar-fix/src/port_corridor_lns.rs`:
Implement:
1. Extraction of cycles from CaDiCaL model.
2. Assumption building:
   - Freeze active edges outside $\Omega$ to True.
   - Forbid inactive edges outside $\Omega$ (set to False).
   - Constrain boundary ports $P_{entry}$ and $P_{exit}$.
3. Mandatory degree-2 clauses for all blocks in $\Omega$.
4. Localized CEGAR loop adding cuts for any cycle strictly internal to $\Omega$.
5. Sequential absorption loop scanning small satellite cycles.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_port_corridor_single_absorb`
Expected: PASS.

- [ ] **Step 5: Run snapshot test to verify cycle absorption on `graph868` snapshot**

Verify on `scratch/graph868_giant_1686_full_edges.txt`.

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/port_corridor_lns.rs src/cegar-fix/tests/test_port_corridor_single_absorb.rs
git commit -m "feat(corridor): implement single-cycle SAT absorption and localized subcycle CEGAR"
```

---

### Task 3: Đấu Nối Vào Vòng Lặp Solver (`hcp_solver.rs`) & Thẩm Định Toàn Bộ Test Suite

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:580-610`
- Test: `src/cegar-fix/tests/test_port_corridor_integration.rs`

**Interfaces:**
- Consumes: `PortCorridorLns::repair`.
- Produces:
  - Automatic online absorption of remaining cycles after `AlternatingPortEngine::repair`.
  - Immediate return when 1 Hamiltonian tour is synthesized.
  - 100% passing across all 55+ test suites (`cargo test --tests`).
  - Clean release compilation (`cargo build --release`).

- [ ] **Step 1: Write integration test**

Create `src/cegar-fix/tests/test_port_corridor_integration.rs` verifying `--alternating-engine` flag invokes `PortCorridorLns` and preserves all CLI outputs.

- [ ] **Step 2: Update `hcp_solver.rs` call site**

In `src/cegar-fix/src/hcp_solver.rs` right after line 583:
```rust
let mut sol_cycles = AlternatingPortEngine::repair(&sol_cycles, &g, contractor, 5, 2000);
if sol_cycles.len() > 1 && !contractor.chain_map.is_empty() {
    let absorbed = PortCorridorLns::repair(&sol_cycles, &g, contractor, encoder, &cnf);
    if absorbed.len() < sol_cycles.len() {
        println!("PortCorridorLns: absorbed satellite subcycles from {} down to {} cycles", sol_cycles.len(), absorbed.len());
        sol_cycles = absorbed;
    }
    if sol_cycles.len() == 1 && (sol_cycles[0].len() == g.adjacency_list.len() || sol_cycles[0].len() == contractor.original_vertices_count) {
        println!("*** 100% HAMILTONIAN TOUR FOUND VIA PORT CORRIDOR LNS! ***");
        let flat: Vec<i32> = sol_cycles.into_iter().flatten().collect();
        let final_tour = if flat.len() == contractor.original_vertices_count {
            flat
        } else {
            contractor.uncontract_cycle(&flat)
        };
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
}
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --tests`
Expected: All 55+ test suites pass.

- [ ] **Step 4: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_port_corridor_integration.rs
git commit -m "feat(solver): wire PortCorridorLns into main CEGAR solver loop"
```

---

### Task 4: Chạy Benchmark Kiểm Chứng Độc Lập 1,800s Trên `graph868.col`

**Files:**
- Execute: `target/release/cegar-fix` via `taskset -c 0,1`
- Verify: `scratch/verify_benchmarks.py`
- Output: `scratch/log_graph868_corridor_1800s.txt`, `scratch/graph868_corridor_found.tour`

- [ ] **Step 1: Copy release binary to `target/release/cegar-fix`**

```bash
mkdir -p target/release && cp src/cegar-fix/target/release/cegar-fix target/release/cegar-fix
```

- [ ] **Step 2: Run benchmark with CPU pinning and timeout 1800s**

```bash
taskset -c 0,1 target/release/cegar-fix FHCPCS-col/graph868.col --timeout 1800 --alternating-engine --output-tour scratch/graph868_corridor_found.tour 2>&1 | tee scratch/log_graph868_corridor_1800s.txt
```

- [ ] **Step 3: Verify Hamiltonian tour if found**

If `scratch/graph868_corridor_found.tour` is produced:
```bash
python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph868.col --tour scratch/graph868_corridor_found.tour
```

- [ ] **Step 4: Document telemetry and report to user**

Update progress ledger and compile comprehensive telemetry report.
