# Staged-Length Lazy SMT Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a high-performance Staged-Length Lazy SMT solver for HCP in `src/cegar-fix` that keeps SAT as the central whole-graph engine while eliminating small cycles incrementally to prevent BCP clause explosion on Universal Core instances like `graph950.col`.

**Architecture:** Whole-graph directed arc SAT formulation using Sinz exact-2 degree counters, pre-pruned 3-cycles, and an incremental CaDiCaL loop with progressive length-staged subcycle filtering ($K_{\text{stage}} \in \{2, 4, 8, 16 \dots\}$) and dual short cut clauses (Direct Exclusion + Boundary Crossing), capped at $\le 500$ clauses/round.

**Tech Stack:** Rust (edition 2021), `rustsat` / `rustsat_cadical` (CaDiCaL backend), `clap`.

## Global Constraints

- Target Graph: `FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$).
- Target Workspace: `/home/ubuntu/HCP/src/cegar-fix`.
- CPU Affinity: Strictly limited to Core 0 & Core 1 (`taskset -c 0,1 nice -n 19`). Leave Core 2 & Core 3 100% free for the user at all times.
- Background Tasks: Exactly 1 active background solver task.
- Zero Tour Injection: Absolutely no reading, importing, or referencing `graph950.hcp.tou`.
- Soundness: Independent raw graph verification (exact-2 degree, 1 single cycle of length $N$, all transitions in $E(G)$).
- Wall-Clock Budget: $\le 1800.0$ seconds.

---

### Task 1: Staged Subcycle Extractor & Length Filter

**Files:**
- Create: `src/cegar-fix/src/staged_subcycle_filter.rs`
- Test: `src/cegar-fix/tests/test_staged_filter.rs`
- Modify: `src/cegar-fix/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Subcycle {
      pub vertices: Vec<i32>,
      pub edges: Vec<(i32, i32)>,
  }

  pub struct StagedSubcycleFilter {
      pub k_stage: usize,
      pub max_batch_size: usize,
  }

  impl StagedSubcycleFilter {
      pub fn new(max_batch_size: usize) -> Self;
      pub fn extract_subcycles(active_arcs: &[(i32, i32)]) -> Vec<Subcycle>;
      pub fn filter_active_cycles<'a>(&mut self, cycles: &'a [Subcycle], n_total: usize) -> Vec<&'a Subcycle>;
  }
  ```

- [ ] **Step 1: Write the failing test**

```rust
// tests/test_staged_filter.rs
use cegar_fix::staged_subcycle_filter::{StagedSubcycleFilter, Subcycle};

#[test]
fn test_staged_subcycle_extraction_and_progression() {
    // 2-factor with one 2-cycle (1<->2) and one 4-cycle (3->4->5->6->3)
    let arcs = vec![
        (1, 2), (2, 1),
        (3, 4), (4, 5), (5, 6), (6, 3),
    ];
    let cycles = StagedSubcycleFilter::extract_subcycles(&arcs);
    assert_eq!(cycles.len(), 2);
    
    let mut filter = StagedSubcycleFilter::new(500);
    assert_eq!(filter.k_stage, 2);

    // Round 1: K_stage = 2 should only select the 2-cycle
    let active = filter.filter_active_cycles(&cycles, 6);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].vertices.len(), 2);

    // Suppose 2-cycle is eliminated, now only 4-cycle exists
    let cycles_round2 = vec![cycles[1].clone()];
    let active2 = filter.filter_active_cycles(&cycles_round2, 6);
    assert_eq!(filter.k_stage, 4);
    assert_eq!(active2.len(), 1);
    assert_eq!(active2[0].vertices.len(), 4);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_staged_filter` in `src/cegar-fix`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `staged_subcycle_filter.rs`**

```rust
// src/staged_subcycle_filter.rs
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Subcycle {
    pub vertices: Vec<i32>,
    pub edges: Vec<(i32, i32)>,
}

pub struct StagedSubcycleFilter {
    pub k_stage: usize,
    pub max_batch_size: usize,
}

impl StagedSubcycleFilter {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            k_stage: 2,
            max_batch_size,
        }
    }

    pub fn extract_subcycles(active_arcs: &[(i32, i32)]) -> Vec<Subcycle> {
        let mut next_map: HashMap<i32, i32> = HashMap::new();
        for &(u, v) in active_arcs {
            next_map.insert(u, v);
        }

        let mut visited: HashSet<i32> = HashSet::new();
        let mut subcycles = Vec::new();

        let mut sorted_vertices: Vec<i32> = next_map.keys().copied().collect();
        sorted_vertices.sort_unstable();

        for &start_v in &sorted_vertices {
            if visited.contains(&start_v) {
                continue;
            }

            let mut curr = start_v;
            let mut cycle_verts = Vec::new();
            let mut cycle_edges = Vec::new();

            while !visited.contains(&curr) {
                visited.insert(curr);
                cycle_verts.push(curr);
                if let Some(&nxt) = next_map.get(&curr) {
                    cycle_edges.push((curr, nxt));
                    curr = nxt;
                } else {
                    break;
                }
            }

            if !cycle_verts.is_empty() {
                subcycles.push(Subcycle {
                    vertices: cycle_verts,
                    edges: cycle_edges,
                });
            }
        }

        subcycles.sort_by_key(|c| c.vertices.len());
        subcycles
    }

    pub fn filter_active_cycles<'a>(
        &mut self,
        cycles: &'a [Subcycle],
        n_total: usize,
    ) -> Vec<&'a Subcycle> {
        if cycles.len() <= 1 {
            return Vec::new();
        }

        loop {
            let mut matches: Vec<&'a Subcycle> = cycles
                .iter()
                .filter(|c| c.vertices.len() <= self.k_stage)
                .collect();

            if !matches.is_empty() {
                if matches.len() > self.max_batch_size {
                    matches.truncate(self.max_batch_size);
                }
                return matches;
            }

            if self.k_stage >= n_total {
                // If stage exceeded N, return the smallest available cycles
                let mut all: Vec<&'a Subcycle> = cycles.iter().collect();
                all.sort_by_key(|c| c.vertices.len());
                if all.len() > self.max_batch_size {
                    all.truncate(self.max_batch_size);
                }
                return all;
            }

            // Advance to next power of 2 stage
            self.k_stage = std::cmp::min(self.k_stage * 2, n_total);
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_staged_filter` in `src/cegar-fix`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/staged_subcycle_filter.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_staged_filter.rs
git commit -m "feat(staged-filter): implement subcycle extractor and staged length filter"
```

---

### Task 2: Dual Short Cut Clause Generator

**Files:**
- Create: `src/cegar-fix/src/dual_cut_generator.rs`
- Test: `src/cegar-fix/tests/test_dual_cut.rs`
- Modify: `src/cegar-fix/src/lib.rs`

**Interfaces:**
- Consumes: `Subcycle` from `staged_subcycle_filter.rs`, `Graph` from `graph.rs`, `Encoder` from `encoder.rs`.
- Produces:
  ```rust
  pub struct DualCutGenerator;

  impl DualCutGenerator {
      pub fn generate_direct_exclusion_clause(
          cycle: &Subcycle,
          encoder: &Encoder,
      ) -> Option<Clause>;

      pub fn generate_boundary_cut_clause(
          cycle: &Subcycle,
          g: &Graph,
          encoder: &Encoder,
      ) -> Option<Clause>;

      pub fn generate_dual_cuts(
          cycle: &Subcycle,
          g: &Graph,
          encoder: &Encoder,
      ) -> Vec<Clause>;
  }
  ```

- [ ] **Step 1: Write the failing test**

```rust
// tests/test_dual_cut.rs
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::staged_subcycle_filter::Subcycle;
use cegar_fix::dual_cut_generator::DualCutGenerator;

#[test]
fn test_dual_cut_generation() {
    let mut g = Graph::new();
    // Triangle (1,2,3) connected to node 4
    g.adjacency_list.insert(1, vec![2, 3]);
    g.adjacency_list.insert(2, vec![1, 3]);
    g.adjacency_list.insert(3, vec![1, 2, 4]);
    g.adjacency_list.insert(4, vec![3]);
    g.update_arcs();

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 1, 0, 0, 0, 0, 0);

    let cycle = Subcycle {
        vertices: vec![1, 2],
        edges: vec![(1, 2), (2, 1)],
    };

    let clauses = DualCutGenerator::generate_dual_cuts(&cycle, &g, &encoder);
    // Should produce 1 direct exclusion clause and 1 boundary cut clause
    assert_eq!(clauses.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_dual_cut` in `src/cegar-fix`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `dual_cut_generator.rs`**

```rust
// src/dual_cut_generator.rs
use std::collections::HashSet;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::staged_subcycle_filter::Subcycle;
use rustsat::types::{Clause, Lit};

pub struct DualCutGenerator;

impl DualCutGenerator {
    pub fn generate_direct_exclusion_clause(
        cycle: &Subcycle,
        encoder: &Encoder,
    ) -> Option<Clause> {
        let mut clause_lits = Vec::with_capacity(cycle.edges.len());
        for &(u, v) in &cycle.edges {
            if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                clause_lits.push(!lit);
            }
        }
        if clause_lits.is_empty() {
            None
        } else {
            clause_lits.sort();
            clause_lits.dedup();
            Some(Clause::from_iter(clause_lits))
        }
    }

    pub fn generate_boundary_cut_clause(
        cycle: &Subcycle,
        g: &Graph,
        encoder: &Encoder,
    ) -> Option<Clause> {
        let cyc_set: HashSet<i32> = cycle.vertices.iter().copied().collect();
        let mut cut_lits: Vec<Lit> = Vec::new();

        for &u in &cycle.vertices {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if !cyc_set.contains(&v) {
                        if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                            cut_lits.push(lit);
                        }
                    }
                }
            }
        }

        if cut_lits.is_empty() {
            None
        } else {
            cut_lits.sort();
            cut_lits.dedup();
            Some(Clause::from_iter(cut_lits))
        }
    }

    pub fn generate_dual_cuts(
        cycle: &Subcycle,
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause> {
        let mut clauses = Vec::with_capacity(2);
        if let Some(excl) = Self::generate_direct_exclusion_clause(cycle, encoder) {
            clauses.push(excl);
        }
        if let Some(boundary) = Self::generate_boundary_cut_clause(cycle, g, encoder) {
            clauses.push(boundary);
        }
        clauses
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_dual_cut` in `src/cegar-fix`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/dual_cut_generator.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_dual_cut.rs
git commit -m "feat(dual-cut): implement direct exclusion and boundary cut generator"
```

---

### Task 3: Staged Lazy SMT Solver Engine & Options Integration

**Files:**
- Create: `src/cegar-fix/src/staged_lazy_smt_solver.rs`
- Modify: `src/cegar-fix/src/options.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct StagedLazySmtOptions {
      pub max_batch_size: usize,
      pub timeout_secs: f64,
      pub output_path: Option<String>,
  }

  pub fn solve_staged_lazy_smt(
      g: &Graph,
      options: &StagedLazySmtOptions,
  ) -> Option<Vec<i32>>;
  ```

- [ ] **Step 1: Write integration test**

```rust
// tests/test_staged_solver.rs
use cegar_fix::graph::Graph;
use cegar_fix::staged_lazy_smt_solver::{solve_staged_lazy_smt, StagedLazySmtOptions};

#[test]
fn test_solve_staged_smt_on_simple_cycle() {
    let mut g = Graph::new();
    // 5-cycle graph: 1-2-3-4-5-1 with chord 1-3
    g.adjacency_list.insert(1, vec![2, 5, 3]);
    g.adjacency_list.insert(2, vec![1, 3]);
    g.adjacency_list.insert(3, vec![2, 4, 1]);
    g.adjacency_list.insert(4, vec![3, 5]);
    g.adjacency_list.insert(5, vec![4, 1]);
    g.update_arcs();

    let options = StagedLazySmtOptions {
        max_batch_size: 500,
        timeout_secs: 10.0,
        output_path: None,
    };

    let tour = solve_staged_lazy_smt(&g, &options);
    assert!(tour.is_some());
    let t = tour.unwrap();
    assert_eq!(t.len(), 5);
}
```

- [ ] **Step 2: Implement `staged_lazy_smt_solver.rs`**

```rust
// src/staged_lazy_smt_solver.rs
use std::time::Instant;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::hcp_solver::add_global_short_cycle_cuts;
use crate::staged_subcycle_filter::StagedSubcycleFilter;
use crate::dual_cut_generator::DualCutGenerator;
use crate::two_tier_orchestrator::{verify_tour_on_raw_graph, write_hcp_tour};
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;

pub struct StagedLazySmtOptions {
    pub max_batch_size: usize,
    pub timeout_secs: f64,
    pub output_path: Option<String>,
}

impl Default for StagedLazySmtOptions {
    fn default() -> Self {
        Self {
            max_batch_size: 500,
            timeout_secs: 1800.0,
            output_path: None,
        }
    }
}

pub fn solve_staged_lazy_smt(
    g: &Graph,
    options: &StagedLazySmtOptions,
) -> Option<Vec<i32>> {
    let start_time = Instant::now();
    let n = g.adjacency_list.len();

    println!("=== Starting Staged-Length Lazy SMT Solver in Rust ===");
    println!("Graph: {} vertices, {} arcs", n, g.arcs.len());

    // 1. Initial Sinz Base CNF Encoding
    let mut encoder = Encoder::new();
    let mut cnf = encoder.encode(g, 1, 0, 0, 0, 0, 0); // -e 1 (Sinz)

    // 2. Pre-prune 3-cycles (triangles)
    let added_triangles = add_global_short_cycle_cuts(g, &encoder, &mut cnf, 3);
    println!(
        "Initial base CNF generated in {:.2}s: {} clauses (pre-pruned {} triangles)",
        start_time.elapsed().as_secs_f64(),
        cnf.len(),
        added_triangles
    );

    // 3. Initialize CaDiCaL
    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    let mut filter = StagedSubcycleFilter::new(options.max_batch_size);
    let mut iteration = 0;
    let mut total_cuts_added = 0;

    while start_time.elapsed().as_secs_f64() < options.timeout_secs {
        iteration += 1;
        let iter_start = Instant::now();

        let res = match solver.solve() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("CaDiCaL solver error at iter {}: {:?}", iteration, e);
                return None;
            }
        };

        let solve_dur = iter_start.elapsed().as_secs_f64();

        match res {
            SolverResult::Sat => {
                // Extract active arcs
                let mut active_arcs = Vec::new();
                for (&(u, v), &lit) in &encoder.graph_lit_map {
                    if let Ok(val) = solver.val(lit) {
                        if val.is_true() {
                            active_arcs.push((u, v));
                        }
                    }
                }

                let subcycles = StagedSubcycleFilter::extract_subcycles(&active_arcs);

                if subcycles.len() == 1 && subcycles[0].vertices.len() == n {
                    let tour = &subcycles[0].vertices;
                    println!(
                        "SUCCESS! Found single Hamiltonian tour with {} vertices at iter {} ({:.2}s total)!",
                        tour.len(),
                        iteration,
                        start_time.elapsed().as_secs_f64()
                    );
                    if verify_tour_on_raw_graph(tour, g) {
                        println!("CERTIFICATION PASSED: Verified tour independently on raw graph G!");
                        let out_path = options
                            .output_path
                            .as_deref()
                            .unwrap_or("scratch/graph950/found_tour_staged_smt.hcp");
                        if let Err(e) = write_hcp_tour(tour, out_path) {
                            eprintln!("Warning: failed to write tour to {}: {}", out_path, e);
                        } else {
                            println!("Wrote certified tour to {}", out_path);
                        }
                        return Some(tour.clone());
                    } else {
                        eprintln!("Verification failed on raw graph.");
                        return None;
                    }
                }

                // Filter candidates matching current K_stage
                let active_cycles = filter.filter_active_cycles(&subcycles, n);

                let mut added_this_round = 0;
                for cyc in &active_cycles {
                    let cuts = DualCutGenerator::generate_dual_cuts(cyc, g, &encoder);
                    for cl in cuts {
                        let _ = solver.add_clause(cl);
                        added_this_round += 1;
                        total_cuts_added += 1;
                    }
                }

                if iteration <= 10 || iteration % 20 == 0 || active_cycles.len() == 1 {
                    println!(
                        "Iter {}: SAT ({:.2}s) | {} subcycles (min len {}, max len {}) | Stage K<={} | Added {} cuts (total {}) | {:.1}s elapsed",
                        iteration,
                        solve_dur,
                        subcycles.len(),
                        subcycles.first().map_or(0, |c| c.vertices.len()),
                        subcycles.last().map_or(0, |c| c.vertices.len()),
                        filter.k_stage,
                        added_this_round,
                        total_cuts_added,
                        start_time.elapsed().as_secs_f64()
                    );
                }
            }
            SolverResult::Unsat => {
                println!(
                    "Solver returned UNSAT at iter {} ({:.2}s). Graph has no Hamiltonian cycle.",
                    iteration,
                    start_time.elapsed().as_secs_f64()
                );
                return None;
            }
            SolverResult::Interrupted => {
                println!("Solver interrupted at iter {}.", iteration);
                return None;
            }
        }
    }

    println!(
        "[TIMEOUT] Reached {:.1}s timeout after {} iterations ({} total cuts added).",
        options.timeout_secs, iteration, total_cuts_added
    );
    None
}
```

- [ ] **Step 3: Update `options.rs` and `main.rs` to support `--staged-smt`**

In `src/cegar-fix/src/options.rs`:
Add `--staged-smt` flag:
```rust
Arg::with_name("staged-smt")
    .long("staged-smt")
    .takes_value(true)
    .default_value("0")
    .help("Staged-length lazy SMT solver:\n 0: Disabled (default)\n 1: Enabled")
```

In `src/cegar-fix/src/main.rs`:
```rust
let staged_smt = matches.value_of_t::<i32>("staged-smt").unwrap_or(0);
if staged_smt == 1 {
    let options = cegar_fix::staged_lazy_smt_solver::StagedLazySmtOptions {
        max_batch_size: 500,
        timeout_secs: timeout,
        output_path: Some(output_tour.to_string()),
    };
    cegar_fix::staged_lazy_smt_solver::solve_staged_lazy_smt(&g, &options);
    return;
}
```

- [ ] **Step 4: Run unit tests to verify all pass**

Run: `cargo test` in `src/cegar-fix`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/ src/cegar-fix/tests/
git commit -m "feat(staged-smt): implement staged-length lazy SMT solver engine and CLI"
```

---

### Task 4: Benchmark Execution & Evaluation on `graph950.col`

**Files:**
- Output: `scratch/graph950/found_tour_staged_smt.hcp`

- [ ] **Step 1: Build release binary**

```bash
taskset -c 0,1 nice -n 19 cargo build --release
```

- [ ] **Step 2: Launch benchmark run**

```bash
taskset -c 0,1 nice -n 19 src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph950.col --staged-smt 1 --timeout 1800 --output-tour scratch/graph950/found_tour_staged_smt.hcp
```

- [ ] **Step 3: Verify execution characteristics**
- Observe per-iteration SAT solve time (confirm $\le 1.5$s per round).
- Observe $K_{\text{stage}}$ progression from $2 \to 4 \to 8 \dots$
- Confirm zero CPU overload on Core 2 & 3.
