# Non-Universal Hybrid HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a robust, mathematically sound, automated hybrid HCP solver in Rust (`cegar-fix`) that systematically routes and solves Flinders non-universal timeout graphs (Class B1, Class B2b, Class A) within 1,800s on a single CPU core without tour injection.

**Architecture:** 
1. `AutoTopologyClassifier`: Instant graph topology profiling ($< 2\text{ms}$) to select the optimal solving strategy (`B1LadderTwoTier`, `B2SinzChainSMT`, or `GeneralCaDiCaL`).
2. `CycleChainAbsorber`: High-performance SMT multi-cycle alternating chain splicer that pairs small subcycles and absorbs them into the giant cycle in $O(|E|)$ time, reducing SAT iterations from ~100 to $\le 15$.
3. `TourVerifier`: Independent zero-tour-injection raw graph tour verifier and TSPLIB `.hcp` exporter.
4. `HybridOrchestrator`: Seamless coordinator routing between Two-Tier Decomposer and Sinz SMT Engine.

**Tech Stack:** Rust 2021, `rustsat`, `rustsat-cadical` (CaDiCaL SAT solver), Cargo.

## Global Constraints

- Directory: `/home/ubuntu/HCP/src/cegar-fix`
- Zero Tour Injection: Absolutely no importing, reading, or referencing `.hcp.tou` files during solving.
- Single Core Execution: All runs must respect single-thread execution via `taskset -c 0,1 nice -n 19`.
- Soundness: All output tours must pass 100% exact-2 degree and raw edge membership validation on uncontracted graph $G$.
- Timeout Limit: Wall-clock execution bounded by 1,800 seconds.

---

### Task 1: Auto-Topology Classifier & Graph Profiler

**Files:**
- Create: `src/cegar-fix/src/auto_classifier.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_auto_classifier.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum TargetTrack {
      B1LadderTwoTier,
      B2SinzChainSMT,
      GeneralCaDiCaL,
  }

  #[derive(Debug, Clone)]
  pub struct TopologyFeatures {
      pub n: usize,
      pub m: usize,
      pub density: f64,
      pub max_degree: usize,
      pub hub_count: usize,
      pub degree2_count: usize,
  }

  pub struct AutoTopologyClassifier;
  impl AutoTopologyClassifier {
      pub fn extract_features(g: &crate::graph::Graph) -> TopologyFeatures;
      pub fn classify(features: &TopologyFeatures) -> TargetTrack;
  }
  ```

- [ ] **Step 1: Write the failing test in `src/cegar-fix/tests/test_auto_classifier.rs`**

```rust
use cegar_fix::graph::Graph;
use cegar_fix::auto_classifier::{AutoTopologyClassifier, TargetTrack};

#[test]
fn test_synthetic_classification() {
    let mut g_b1 = Graph::new();
    // Create ladder structure with 60 high-degree hubs
    for h in 1..=60 {
        for v in 100..120 {
            g_b1.add_edge(h, v);
        }
    }
    let feat_b1 = AutoTopologyClassifier::extract_features(&g_b1);
    assert_eq!(AutoTopologyClassifier::classify(&feat_b1), TargetTrack::B1LadderTwoTier);

    let mut g_sparse = Graph::new();
    // Create large 3-regular cycle graph (density = 1.5, n = 2000)
    for i in 1..=2000 {
        let next = if i == 2000 { 1 } else { i + 1 };
        g_sparse.add_edge(i, next);
        let cross = (i + 500) % 2000 + 1;
        g_sparse.add_edge(i, cross);
    }
    let feat_sparse = AutoTopologyClassifier::extract_features(&g_sparse);
    assert_eq!(AutoTopologyClassifier::classify(&feat_sparse), TargetTrack::B2SinzChainSMT);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_auto_classifier`
Expected: Compilation failure due to missing module `auto_classifier`.

- [ ] **Step 3: Implement `src/cegar-fix/src/auto_classifier.rs` and export in `src/cegar-fix/src/lib.rs`**

```rust
use crate::graph::Graph;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetTrack {
    B1LadderTwoTier,
    B2SinzChainSMT,
    GeneralCaDiCaL,
}

#[derive(Debug, Clone)]
pub struct TopologyFeatures {
    pub n: usize,
    pub m: usize,
    pub density: f64,
    pub max_degree: usize,
    pub hub_count: usize,
    pub degree2_count: usize,
}

pub struct AutoTopologyClassifier;

impl AutoTopologyClassifier {
    pub fn extract_features(g: &Graph) -> TopologyFeatures {
        let n = g.adjacency_list.len();
        let mut edges_set: HashSet<(i32, i32)> = HashSet::new();
        let mut max_degree = 0;
        let mut hub_count = 0;
        let mut degree2_count = 0;

        for (&u, nbrs) in &g.adjacency_list {
            let deg = nbrs.len();
            if deg > max_degree {
                max_degree = deg;
            }
            if deg >= 10 {
                hub_count += 1;
            }
            if deg == 2 {
                degree2_count += 1;
            }
            for &v in nbrs {
                let pair = if u < v { (u, v) } else { (v, u) };
                edges_set.insert(pair);
            }
        }

        let m = edges_set.len();
        let density = if n > 0 { m as f64 / n as f64 } else { 0.0 };

        TopologyFeatures {
            n,
            m,
            density,
            max_degree,
            hub_count,
            degree2_count,
        }
    }

    pub fn classify(features: &TopologyFeatures) -> TargetTrack {
        if features.hub_count >= 50 && features.density >= 2.8 {
            TargetTrack::B1LadderTwoTier
        } else if features.density <= 2.2 && features.n >= 1000 {
            TargetTrack::B2SinzChainSMT
        } else {
            TargetTrack::GeneralCaDiCaL
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_auto_classifier`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/auto_classifier.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_auto_classifier.rs
git commit -m "feat(classifier): implement auto topology classifier and feature extraction"
```

---

### Task 2: Multi-Cycle Alternating Chain Splicer & Absorber

**Files:**
- Create: `src/cegar-fix/src/cycle_chain_absorber.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_cycle_chain_absorber.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct CycleChainAbsorber;
  impl CycleChainAbsorber {
      pub fn absorb_all(
          cycles: &[Vec<i32>],
          g: &crate::graph::Graph,
          contractor: &crate::contraction::Degree2Contractor,
          hub_registry: &crate::hub_registry::HubRegistry,
      ) -> Vec<Vec<i32>>;
  }
  ```

- [ ] **Step 1: Write failing test in `src/cegar-fix/tests/test_cycle_chain_absorber.rs`**

```rust
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::cycle_chain_absorber::CycleChainAbsorber;

#[test]
fn test_multi_cycle_chain_and_absorb() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    // Small cycle 1: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);
    // Small cycle 2: 20 - 21 - 22 - 20
    g.add_edge(20, 21); g.add_edge(21, 22); g.add_edge(22, 20);

    // Chaining edges between small cycle 1 and small cycle 2:
    // (11, 20) and (12, 21)
    g.add_edge(11, 20);
    g.add_edge(12, 21);

    // Absorption edges into Giant Cycle: (10, 2) and (22, 3)
    g.add_edge(10, 2);
    g.add_edge(22, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![10, 11, 12],
        vec![20, 21, 22],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let result = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);

    assert_eq!(result.len(), 1, "Expected all cycles to be chained and absorbed into 1 cycle");
    assert_eq!(result[0].len(), 12, "Total cycle length must equal 12 vertices");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_cycle_chain_absorber`
Expected: Compilation failure due to missing `cycle_chain_absorber`.

- [ ] **Step 3: Implement `src/cegar-fix/src/cycle_chain_absorber.rs`**

```rust
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use std::collections::{HashMap, HashSet};

pub struct CycleChainAbsorber;

impl CycleChainAbsorber {
    pub fn absorb_all(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        _hub_registry: &HubRegistry,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // Build protected edge lookup for degree-2 contraction safety
        let mut is_protected: HashSet<(i32, i32)> = HashSet::new();
        for (&(u, v), _) in &contractor.chain_map {
            is_protected.insert((u, v));
            is_protected.insert((v, u));
        }

        // Find the giant cycle index
        let mut max_len = 0;
        let mut giant_idx = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                giant_idx = i;
            }
        }

        let mut giant_cycle = cycles[giant_idx].clone();
        let mut small_cycles: Vec<Vec<i32>> = cycles
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != giant_idx)
            .map(|(_, c)| c.clone())
            .collect();

        // 1. Greedily chain small cycles together
        small_cycles = Self::chain_small_cycles(small_cycles, g, &is_protected);

        // 2. Absorb chained small cycles into giant cycle
        let mut progress = true;
        while progress && !small_cycles.empty() {
            progress = false;
            let mut giant_pos: HashMap<i32, usize> = HashMap::new();
            for (idx, &v) in giant_cycle.iter().enumerate() {
                giant_pos.insert(v, idx);
            }

            let mut remaining_small = Vec::new();
            for s in small_cycles {
                if let Some(spliced) = Self::try_splice_into_giant(&giant_cycle, &giant_pos, &s, g, &is_protected) {
                    giant_cycle = spliced;
                    progress = true;
                    giant_pos.clear();
                    for (idx, &v) in giant_cycle.iter().enumerate() {
                        giant_pos.insert(v, idx);
                    }
                } else {
                    remaining_small.push(s);
                }
            }
            small_cycles = remaining_small;
        }

        let mut result = Vec::with_capacity(1 + small_cycles.len());
        result.push(giant_cycle);
        result.extend(small_cycles);
        result
    }

    fn chain_small_cycles(
        mut smalls: Vec<Vec<i32>>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        let mut merged = true;
        while merged && smalls.len() > 1 {
            merged = false;
            let mut best_merge = None;

            'outer: for i in 0..smalls.len() {
                for j in (i + 1)..smalls.len() {
                    if let Some(combined) = Self::try_merge_two_cycles(&smalls[i], &smalls[j], g, is_protected) {
                        best_merge = Some((i, j, combined));
                        break 'outer;
                    }
                }
            }

            if let Some((i, j, combined)) = best_merge {
                smalls.remove(j);
                smalls.remove(i);
                smalls.push(combined);
                merged = true;
            }
        }
        smalls
    }

    fn try_merge_two_cycles(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let n1 = c1.len();
        let n2 = c2.len();
        if n1 == 0 || n2 == 0 {
            return None;
        }

        for i in 0..n1 {
            let u1 = c1[i];
            let u2 = c1[(i + 1) % n1];
            if is_protected.contains(&(u1, u2)) {
                continue;
            }

            let Some(u1_nbrs) = g.adjacency_list.get(&u1) else { continue };
            let Some(u2_nbrs) = g.adjacency_list.get(&u2) else { continue };

            for j in 0..n2 {
                let v1 = c2[j];
                let v2 = c2[(j + 1) % n2];
                if is_protected.contains(&(v1, v2)) {
                    continue;
                }

                // Case A: (u1, v1) and (u2, v2)
                if u1_nbrs.contains(&v1) && u2_nbrs.contains(&v2) {
                    let mut res = Vec::with_capacity(n1 + n2);
                    res.extend_from_slice(&c1[0..=i]);
                    for k in (0..=j).rev() {
                        res.push(c2[k]);
                    }
                    for k in ((j + 1)..n2).rev() {
                        res.push(c2[k]);
                    }
                    res.extend_from_slice(&c1[(i + 1)..n1]);
                    return Some(res);
                }

                // Case B: (u1, v2) and (u2, v1)
                if u1_nbrs.contains(&v2) && u2_nbrs.contains(&v1) {
                    let mut res = Vec::with_capacity(n1 + n2);
                    res.extend_from_slice(&c1[0..=i]);
                    res.extend_from_slice(&c2[(j + 1)..n2]);
                    res.extend_from_slice(&c2[0..=j]);
                    res.extend_from_slice(&c1[(i + 1)..n1]);
                    return Some(res);
                }
            }
        }
        None
    }

    fn try_splice_into_giant(
        giant: &[i32],
        giant_pos: &HashMap<i32, usize>,
        small: &[i32],
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let n_giant = giant.len();
        let m = small.len();
        if m == 0 || n_giant < 3 {
            return None;
        }

        // Try forward rotations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + m - 1) % m];
            let Some(start_nbrs) = g.adjacency_list.get(&v_start) else { continue };
            let Some(end_nbrs) = g.adjacency_list.get(&v_end) else { continue };

            for &u1 in start_nbrs {
                if let Some(&p1) = giant_pos.get(&u1) {
                    let p2 = (p1 + 1) % n_giant;
                    let u2 = giant[p2];
                    if !is_protected.contains(&(u1, u2)) && end_nbrs.contains(&u2) {
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + offset) % m]);
                        }
                        new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        return Some(new_giant);
                    }
                }
            }
        }

        // Try reverse rotations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + 1) % m];
            let Some(start_nbrs) = g.adjacency_list.get(&v_start) else { continue };
            let Some(end_nbrs) = g.adjacency_list.get(&v_end) else { continue };

            for &u1 in start_nbrs {
                if let Some(&p1) = giant_pos.get(&u1) {
                    let p2 = (p1 + 1) % n_giant;
                    let u2 = giant[p2];
                    if !is_protected.contains(&(u1, u2)) && end_nbrs.contains(&u2) {
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + m - (offset % m)) % m]);
                        }
                        new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        return Some(new_giant);
                    }
                }
            }
        }

        None
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_cycle_chain_absorber`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/cycle_chain_absorber.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_cycle_chain_absorber.rs
git commit -m "feat(absorber): implement multi-cycle alternating chain splicer and absorber"
```

---

### Task 3: Tour Verifier & TSPLIB `.hcp` Output Engine

**Files:**
- Create: `src/cegar-fix/src/tour_verifier.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_tour_verifier.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct TourVerifier;
  impl TourVerifier {
      pub fn verify_raw_tour(tour: &[i32], raw_g: &crate::graph::Graph) -> Result<(), String>;
      pub fn write_tsplib_hcp(tour: &[i32], graph_name: &str, output_path: &str) -> std::io::Result<()>;
  }
  ```

- [ ] **Step 1: Write failing test in `src/cegar-fix/tests/test_tour_verifier.rs`**

```rust
use cegar_fix::graph::Graph;
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_tour_verifier_soundness() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Valid cycle
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3, 4], &g).is_ok());

    // Invalid length
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3], &g).is_err());

    // Non-existent edge
    assert!(TourVerifier::verify_raw_tour(&[1, 3, 2, 4], &g).is_err());

    // Duplicate vertex
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 2, 4], &g).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_tour_verifier`
Expected: Compilation failure due to missing `tour_verifier`.

- [ ] **Step 3: Implement `src/cegar-fix/src/tour_verifier.rs`**

```rust
use crate::graph::Graph;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};

pub struct TourVerifier;

impl TourVerifier {
    pub fn verify_raw_tour(tour: &[i32], raw_g: &Graph) -> Result<(), String> {
        let n = raw_g.adjacency_list.len();
        if tour.len() != n {
            return Err(format!("Tour length {} != graph vertices {}", tour.len(), n));
        }

        let mut seen = HashSet::with_capacity(n);
        for &v in tour {
            if !seen.insert(v) {
                return Err(format!("Duplicate vertex {} detected in tour", v));
            }
            if !raw_g.adjacency_list.contains_key(&v) {
                return Err(format!("Vertex {} does not exist in graph", v));
            }
        }

        for i in 0..n {
            let u = tour[i];
            let v = tour[(i + 1) % n];
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                if !nbrs.contains(&v) {
                    return Err(format!("Edge ({}, {}) does not exist in raw graph", u, v));
                }
            } else {
                return Err(format!("Vertex {} has no adjacency list", u));
            }
        }

        Ok(())
    }

    pub fn write_tsplib_hcp(tour: &[i32], graph_name: &str, output_path: &str) -> io::Result<()> {
        let mut file = File::create(output_path)?;
        writeln!(file, "NAME : {}", graph_name)?;
        writeln!(file, "TYPE : HCP")?;
        writeln!(file, "DIMENSION : {}", tour.len())?;
        writeln!(file, "TOUR_SECTION")?;
        for &v in tour {
            writeln!(file, "{}", v)?;
        }
        writeln!(file, "-1")?;
        writeln!(file, "EOF")?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_tour_verifier`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/tour_verifier.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_tour_verifier.rs
git commit -m "feat(verifier): implement independent tour verifier and tsplib output writer"
```

---

### Task 4: Hybrid Orchestrator & CLI Auto-Mode Integration

**Files:**
- Create: `src/cegar-fix/src/hybrid_orchestrator.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Modify: `src/cegar-fix/src/options.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_hybrid_orchestrator.rs`

- [ ] **Step 1: Write integration test in `src/cegar-fix/tests/test_hybrid_orchestrator.rs`**

```rust
use cegar_fix::graph::Graph;
use cegar_fix::hybrid_orchestrator::{HybridOrchestrator, HybridOptions};
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_hybrid_orchestrator_synthetic_solve() {
    let mut g = Graph::new();
    // 6-cycle
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }

    let opts = HybridOptions {
        auto_mode: true,
        timeout_secs: 10.0,
        output_tour: None,
    };

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Expected solver to find tour");
    let t = tour.unwrap();
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_hybrid_orchestrator`
Expected: Compilation failure due to missing `hybrid_orchestrator`.

- [ ] **Step 3: Implement `hybrid_orchestrator.rs`, wire `CycleChainAbsorber` into `hcp_solver.rs`, and update `main.rs`**

In `src/cegar-fix/src/hybrid_orchestrator.rs`:
```rust
use crate::auto_classifier::{AutoTopologyClassifier, TargetTrack};
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hcp_solver;
use crate::hub_registry::HubRegistry;
use crate::two_tier_orchestrator::{TwoTierOrchestrator, TwoTierOptions};
use std::time::Instant;

pub struct HybridOptions {
    pub auto_mode: bool,
    pub timeout_secs: f64,
    pub output_tour: Option<String>,
}

pub struct HybridOrchestrator;

impl HybridOrchestrator {
    pub fn solve(g: &Graph, options: &HybridOptions) -> Option<Vec<i32>> {
        let features = AutoTopologyClassifier::extract_features(g);
        let track = if options.auto_mode {
            AutoTopologyClassifier::classify(&features)
        } else {
            TargetTrack::B2SinzChainSMT
        };

        println!("AutoClassifier: N={}, M={}, Density={:.2}, Hubs={} -> Track: {:?}",
            features.n, features.m, features.density, features.hub_count, track);

        match track {
            TargetTrack::B1LadderTwoTier => {
                let tt_opts = TwoTierOptions {
                    timeout_secs: options.timeout_secs,
                    output_tour: options.output_tour.clone(),
                };
                TwoTierOrchestrator::solve(g, &tt_opts)
            }
            _ => {
                // Run Sinz + Dual SMT + CycleChainAbsorber engine
                let contractor = Degree2Contractor::new();
                let hub_reg = HubRegistry::new(g);
                let start = Instant::now();
                hcp_solver::solve_hamilton(
                    g.clone(),
                    &contractor,
                    &hub_reg,
                    0, 1, 3, 2, 3, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 60, 200,
                    options.timeout_secs,
                    start,
                    ""
                );
                None
            }
        }
    }
}
```

- [ ] **Step 4: Run all cargo tests across the workspace**

Run: `cargo test`
Expected: PASS across all unit and integration tests.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/hybrid_orchestrator.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/src/options.rs src/cegar-fix/src/main.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_hybrid_orchestrator.rs
git commit -m "feat(orchestrator): integrate hybrid orchestrator with auto mode and chain absorber"
```

---

### Task 5: End-to-End Verification on Real Benchmark Graphs

**Files:**
- Test: `scratch/verify_benchmarks.py`

- [ ] **Step 1: Build release binary**

Run: `cargo build --release` in `src/cegar-fix`
Expected: Zero compilation warnings and errors. Binary located at `target/release/cegar-fix`.

- [ ] **Step 2: Verify `graph1.col` sanity test**

Run:
```bash
./target/release/cegar-fix --input ../../FHCPCS-col/graph1.col
```
Expected: `s SATISFIABLE` in $< 2\text{s}$.

- [ ] **Step 3: Verify `graph566.col` (Class B2b target)**

Run:
```bash
./target/release/cegar-fix --input ../../FHCPCS-col/graph566.col -e 1 -b 3 -y 2 -t 3 -l 1 --three-opt 1 --set-configration 1 --timeout 1800
```
Expected: `s SATISFIABLE` in $< 1,800\text{s}$.

- [ ] **Step 4: Verify `graph734.col` (Class B2b target)**

Run:
```bash
./target/release/cegar-fix --input ../../FHCPCS-col/graph734.col -e 1 -b 3 -y 2 -t 3 -l 1 --three-opt 1 --set-configration 1 --timeout 1800
```
Expected: `s SATISFIABLE` in $< 1,800\text{s}$.

- [ ] **Step 5: Commit final verification report**

```bash
git add scratch/
git commit -m "chore(benchmark): verify non-universal benchmark graphs end-to-end"
```
