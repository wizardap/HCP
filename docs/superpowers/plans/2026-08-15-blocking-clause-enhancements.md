# Enhanced Blocking Clauses (Techniques A1, A2, A3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement three complementary mathematical enhancements to CEGAR blocking clauses (Boundary Minimal Cut Reduction, Induced Subgraph SECs, Complementary Cut Symmetry) to eliminate solver stalls on dense hub graphs and ping-pong loops on large sparse graphs.

**Architecture:** Refactor `get_blocking_clauses` in `src/cegar-fix/src/hcp_solver.rs` to generate boundary-filtered cuts via $O(1)$ set lookups, construct complementary cuts when $|C| > |V|/2$, and generate induced subgraph subtour elimination clauses for small cycles ($|C| \le 6$) with chords.

**Tech Stack:** Rust, `rustsat`, `rustsat-cadical` (CaDiCaL SAT Solver).

## Global Constraints
- Must preserve 100% zero regressions against the official author's baseline results.
- Must strictly maintain standard command-line compatibility (`-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`).
- All code changes are isolated to `src/cegar-fix/src/hcp_solver.rs`.

---

### Task 1: Technique A1 & A3 — Boundary Minimal Cut & Complementary Cut Symmetry

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/src/hcp_solver.rs` (unit tests at bottom of file)

**Interfaces:**
- Produces:
  ```rust
  pub fn get_boundary_cut_clauses(
      cycle: &[i32],
      encoder: &mut Encoder,
      g: &Graph,
      total_vertices: usize,
  ) -> Vec<Clause>
  ```

- [ ] **Step 1: Write unit tests for Boundary Minimal Cut and Complementary Cut Symmetry**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
#[cfg(test)]
mod tests_blocking_enhancements {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_boundary_cut_complementary_equivalence() {
        // Build a 6-vertex cycle graph 1-2-3-4-5-6-1 with a chord 1-4
        let mut adj = BTreeMap::new();
        adj.insert(1, vec![2, 6, 4]);
        adj.insert(2, vec![1, 3]);
        adj.insert(3, vec![2, 4]);
        adj.insert(4, vec![3, 5, 1]);
        adj.insert(5, vec![4, 6]);
        adj.insert(6, vec![5, 1]);
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: vec![],
        };
        let mut encoder = Encoder::new();
        encoder.encode_graph(&g);

        // Subcycle C = [1, 2, 3, 4] (|C| = 4 > 6/2 = 3) -> Complementary S = [5, 6]
        let c = vec![1, 2, 3, 4];
        let clauses = get_boundary_cut_clauses(&c, &mut encoder, &g, 6);
        assert!(!clauses.is_empty());
        // Verify both out-cut and in-cut clauses exist
        assert_eq!(clauses.len(), 2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_boundary_cut_complementary_equivalence`
Expected: FAIL (function `get_boundary_cut_clauses` does not exist).

- [ ] **Step 3: Implement `get_boundary_cut_clauses`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
pub fn get_boundary_cut_clauses(
    cycle: &[i32],
    encoder: &mut Encoder,
    g: &Graph,
    total_vertices: usize,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let cycle_set: HashSet<i32> = cycle.iter().cloned().collect();

    // Technique A3: If |C| > |V| / 2, use complementary set S = V \ C
    let (target_set, is_complementary) = if cycle.len() > total_vertices / 2 && total_vertices > 0 {
        let all_v: HashSet<i32> = g.adjacency_list.keys().cloned().collect();
        let comp_set: HashSet<i32> = all_v.difference(&cycle_set).cloned().collect();
        (comp_set, true)
    } else {
        (cycle_set, false)
    };

    let mut clause_out = rustsat::types::Clause::new();
    let mut clause_in = rustsat::types::Clause::new();

    // Technique A1: Iterate over vertices in target_set and only collect boundary cut edges
    for &u in &target_set {
        if let Some(adjs) = g.adjacency_list.get(&u) {
            for &v in adjs {
                if !target_set.contains(&v) {
                    if let Some(lit_out) = encoder.graph_lit_map.get(&(u, v)) {
                        clause_out.add(*lit_out);
                    }
                    if let Some(lit_in) = encoder.graph_lit_map.get(&(v, u)) {
                        clause_in.add(*lit_in);
                    }
                }
            }
        }
    }

    if is_complementary {
        // By duality: delta^+(V \ C) = delta^-(C) and delta^-(V \ C) = delta^+(C)
        if !clause_in.is_empty() {
            clauses.push(clause_in);
        }
        if !clause_out.is_empty() {
            clauses.push(clause_out);
        }
    } else {
        if !clause_out.is_empty() {
            clauses.push(clause_out);
        }
        if !clause_in.is_empty() {
            clauses.push(clause_in);
        }
    }

    clauses
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_boundary_cut_complementary_equivalence`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement boundary minimal cut and complementary cut symmetry (A1, A3)"
```

---

### Task 2: Technique A2 — Induced Subgraph SECs (Subtour Elimination Constraints)

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/src/hcp_solver.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn get_induced_subgraph_sec_clauses(
      cycle: &[i32],
      encoder: &Encoder,
      g: &Graph,
  ) -> Vec<Clause>
  ```

- [ ] **Step 1: Write unit tests for Induced Subgraph SECs**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
#[test]
fn test_induced_subgraph_chord_cycles() {
    // 4-vertex graph 1-2-3-4 with chord (1,3)
    let mut adj = BTreeMap::new();
    adj.insert(1, vec![2, 4, 3]);
    adj.insert(2, vec![1, 3]);
    adj.insert(3, vec![2, 4, 1]);
    adj.insert(4, vec![3, 1]);
    let g = Graph {
        adjacency_list: adj,
        adjacency_list_btree: BTreeMap::new(),
        arcs: vec![],
    };
    let mut encoder = Encoder::new();
    encoder.encode_graph(&g);

    let c = vec![1, 2, 3, 4];
    let clauses = get_induced_subgraph_sec_clauses(&c, &encoder, &g);
    // Must generate at least the main cycle exclusion + chord triangle exclusion
    assert!(!clauses.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_induced_subgraph_chord_cycles`
Expected: FAIL.

- [ ] **Step 3: Implement `get_induced_subgraph_sec_clauses`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
pub fn get_induced_subgraph_sec_clauses(
    cycle: &[i32],
    encoder: &Encoder,
    g: &Graph,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let len = cycle.len();
    if len < 3 || len > 6 {
        return clauses;
    }

    let cycle_set: HashSet<i32> = cycle.iter().cloned().collect();

    // 1. Standard forward & reverse exclusion for active cycle C
    let mut fwd_clause = rustsat::types::Clause::new();
    for i in 0..len {
        if let Some(lit) = encoder.graph_lit_map.get(&(cycle[i], cycle[(i + 1) % len])) {
            fwd_clause.add(!*lit);
        }
    }
    if !fwd_clause.is_empty() {
        clauses.push(fwd_clause);
    }

    if len != 2 {
        let mut rev_clause = rustsat::types::Clause::new();
        for i in (0..len).rev() {
            if let Some(lit) = encoder.graph_lit_map.get(&(cycle[i], cycle[(i + len - 1) % len])) {
                rev_clause.add(!*lit);
            }
        }
        if !rev_clause.is_empty() {
            clauses.push(rev_clause);
        }
    }

    // 2. Search for internal chords in G[C] to forbid chord subtours
    // For small |C| <= 6, enumerate chord paths
    for i in 0..len {
        let u = cycle[i];
        if let Some(adjs) = g.adjacency_list.get(&u) {
            for &v in adjs {
                if cycle_set.contains(&v) {
                    let next_u = cycle[(i + 1) % len];
                    let prev_u = cycle[(i + len - 1) % len];
                    // If (u, v) is a chord (not consecutive in C)
                    if v != next_u && v != prev_u {
                        // Forbid shortcut triangle (u, next_u, v) if (next_u, v) is an edge
                        if let Some(next_adjs) = g.adjacency_list.get(&next_u) {
                            if next_adjs.contains(&v) {
                                if let (Some(l1), Some(l2), Some(l3)) = (
                                    encoder.graph_lit_map.get(&(u, next_u)),
                                    encoder.graph_lit_map.get(&(next_u, v)),
                                    encoder.graph_lit_map.get(&(v, u)),
                                ) {
                                    clauses.push(clause!(!*l1, !*l2, !*l3));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    clauses
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_induced_subgraph_chord_cycles`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement induced subgraph SECs for small subcycles (A2)"
```

---

### Task 3: Integration into `get_blocking_clauses` & CEGAR Pipeline

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`

**Interfaces:**
- Updates `get_blocking_clauses` to seamlessly integrate A1, A2, and A3 under `block_method == 3`.

- [ ] **Step 1: Update `get_blocking_clauses` in `src/cegar-fix/src/hcp_solver.rs`**

Refactor `get_blocking_clauses`:
```rust
fn get_blocking_clauses(
    sol_cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    block_method: i32,
    balanced: i32,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let total_v = g.adjacency_list.len();

    for sol_cycle in sol_cycles.iter() {
        match block_method {
            3 => {
                // Technique A1 & A3: Boundary Minimal Cut & Complementary Cut
                let cut_clauses = get_boundary_cut_clauses(sol_cycle, encoder, g, total_v);
                clauses.extend(cut_clauses);

                // Technique A2: Induced Subgraph SECs for |C| <= 6
                if sol_cycle.len() <= 6 {
                    let sec_clauses = get_induced_subgraph_sec_clauses(sol_cycle, encoder, g);
                    clauses.extend(sec_clauses);
                }
            }
            0 => clauses.extend(cegar_blocking_clauses(sol_cycle, &encoder.graph_lit_map)),
            1 => clauses.extend(asp_blocking_clauses(sol_cycle, encoder, g, 1, balanced)),
            _ => clauses.extend(asp_blocking_clauses(sol_cycle, encoder, g, 2, balanced)),
        }
    }
    clauses
}
```

- [ ] **Step 2: Build release binary and run cargo tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate A1, A2, A3 into get_blocking_clauses"
```

---

### Task 4: Regression Verification & Performance Benchmark

**Files:**
- Test: Benchmark execution on standard and timeout testsets.

- [ ] **Step 1: Verify 10 Key Regression Graphs**

Run verification across known benchmark graphs:
```bash
for g in graph45 graph132 graph161 graph178 graph183 graph230 graph248 graph313 graph339; do
    echo -n "$g: "
    ./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/${g}.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1 2>&1 | grep -E "s SATIS|overall time" | tr '\n' ' '
    echo ""
done
```
Expected: All 9 graphs finish with `s SATISFIABLE`.

- [ ] **Step 2: Profile on Dense Hub Timeout Graph `graph560.col`**

Run: `timeout 60 ./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1 2>&1 | grep -E "s SATIS|increment time|overall time"`
Observe propagation speed and increment times.

- [ ] **Step 3: Commit benchmark results and update progress ledger**

```bash
git add .superpowers/sdd/
git commit -m "docs: record verification results for enhanced blocking clauses"
```
