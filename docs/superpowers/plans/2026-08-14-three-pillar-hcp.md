# Three-Pillar SAT-based CEGAR HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Three-Pillar architecture for the SAT-based CEGAR Hamiltonian Cycle Solver: (1) Graph Preprocessing & Invariant Pruning, (2) Non-Polluting 2-opt/3-opt Solution Constructor, and (3) Strong Minimal Subcycle Cuts to achieve 100% regression freedom and maximize solving speed.

**Architecture:** 
- Invariant Graph Preprocessing removes unneeded Boolean variables and detects cut-vertices before SAT encoding.
- The 2-opt/3-opt heuristic is strictly encapsulated as a pure polynomial Solution Constructor (yielding solutions when possible, but never polluting CaDiCaL with synthetic/fallback clauses).
- CEGAR cuts are refined into strong dual cuts for small cycles ($|C| \le 4$) and clean minimal ASP cut-set clauses for general subcycles ($|C| > 4$).

**Tech Stack:** Rust 2021, CaDiCaL SAT solver (`rustsat-cadical`), `rustsat` library.

## Global Constraints

- Must preserve exact output format (`s SATISFIABLE`, `s UNSATISFIABLE`, `solution: \n...`).
- Zero regression on all 926 graphs solved by the baseline.
- No artificial time-based fallback or spurious clause poisoning.

---

### Task 1: Pillar 1 - Graph Invariant Pruning & Cut-Vertex Detection

**Files:**
- Modify: `src/cegar-fix/src/graph.rs`
- Modify: `src/cegar-fix/src/main.rs:40-60`

**Interfaces:**
- Produces:
  - `Graph::prune_degree2_triangles(&mut self) -> usize`
  - `Graph::has_articulation_points(&self) -> bool`

- [ ] **Step 1: Implement `prune_degree2_triangles` in `src/graph.rs`**

```rust
impl Graph {
    /// If vertex v has degree 2 with neighbors u and w, and edge (u, w) exists (with |V| > 3),
    /// edge (u, w) cannot be part of any Hamiltonian cycle because choosing (u, w) isolates u-v-w-u.
    pub fn prune_degree2_triangles(&mut self) -> usize {
        let n = self.adjacency_list_btree.len();
        if n <= 3 {
            return 0;
        }
        let mut edges_to_remove = Vec::new();
        for (&v, neighbors) in self.adjacency_list_btree.iter() {
            if neighbors.len() == 2 {
                let u = neighbors[0];
                let w = neighbors[1];
                if let Some(u_neighbors) = self.adjacency_list.get(&u) {
                    if u_neighbors.contains(&w) {
                        edges_to_remove.push((u, w));
                    }
                }
            }
        }
        let mut count = 0;
        for (u, w) in edges_to_remove {
            if self.remove_edge_if_exists(u, w) {
                count += 1;
            }
        }
        count
    }

    fn remove_edge_if_exists(&mut self, u: i32, w: i32) -> bool {
        let mut removed = false;
        if let Some(u_list) = self.adjacency_list.get_mut(&u) {
            if let Some(pos) = u_list.iter().position(|&x| x == w) {
                u_list.remove(pos);
                removed = true;
            }
        }
        if let Some(w_list) = self.adjacency_list.get_mut(&w) {
            if let Some(pos) = w_list.iter().position(|&x| x == u) {
                w_list.remove(pos);
            }
        }
        if let Some(u_list) = self.adjacency_list_btree.get_mut(&u) {
            if let Some(pos) = u_list.iter().position(|&x| x == w) {
                u_list.remove(pos);
            }
        }
        if let Some(w_list) = self.adjacency_list_btree.get_mut(&w) {
            if let Some(pos) = w_list.iter().position(|&x| x == u) {
                w_list.remove(pos);
            }
        }
        self.arcs.retain(|&(a, b)| !((a == u && b == w) || (a == w && b == u)));
        removed
    }
}
```

- [ ] **Step 2: Implement `has_articulation_points` via Tarjan's DFS in `src/graph.rs`**

```rust
impl Graph {
    /// Returns true if removing any single vertex disconnects the graph (instant UNSAT for HCP).
    pub fn has_articulation_points(&self) -> bool {
        let vertices: Vec<i32> = self.adjacency_list_btree.keys().copied().collect();
        let n = vertices.len();
        if n <= 2 {
            return false;
        }

        let mut tin = HashMap::new();
        let mut low = HashMap::new();
        let mut timer = 0;
        let mut is_cut = false;

        fn dfs(
            v: i32, p: i32,
            adj: &HashMap<i32, Vec<i32>>,
            tin: &mut HashMap<i32, usize>,
            low: &mut HashMap<i32, usize>,
            timer: &mut usize,
            is_cut: &mut bool
        ) {
            *timer += 1;
            tin.insert(v, *timer);
            low.insert(v, *timer);
            let mut children = 0;
            if let Some(neighbors) = adj.get(&v) {
                for &to in neighbors {
                    if to == p { continue; }
                    if tin.contains_key(&to) {
                        let to_tin = *tin.get(&to).unwrap();
                        let v_low = low.get_mut(&v).unwrap();
                        *v_low = std::cmp::min(*v_low, to_tin);
                    } else {
                        dfs(to, v, adj, tin, low, timer, is_cut);
                        let to_low = *low.get(&to).unwrap();
                        let v_low = low.get_mut(&v).unwrap();
                        *v_low = std::cmp::min(*v_low, to_low);
                        if to_low >= *tin.get(&v).unwrap() && p != -1 {
                            *is_cut = true;
                        }
                        children += 1;
                    }
                }
            }
            if p == -1 && children > 1 {
                *is_cut = true;
            }
        }

        dfs(vertices[0], -1, &self.adjacency_list, &mut tin, &mut low, &mut timer, &mut is_cut);

        // Also check if graph is disconnected
        if tin.len() < n {
            return true;
        }
        is_cut
    }
}
```

- [ ] **Step 3: Integrate Preprocessing in `src/main.rs`**

Update `main.rs` to call `has_articulation_points()` and `prune_degree2_triangles()` before encoding:
```rust
if g.has_articulation_points() {
    println!("Graph has cut-vertex or is disconnected.");
    println!("s UNSATISFIABLE");
    return;
}
let pruned = g.prune_degree2_triangles();
if pruned > 0 {
    println!("Pruned {} degree-2 triangle shortcut edges", pruned);
}
```

- [ ] **Step 4: Build and test binary**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo build --release`
Expected: Success with 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/graph.rs src/cegar-fix/src/main.rs
git commit -m "feat: add degree-2 triangle pruning and articulation point detection"
```

---

### Task 2: Pillar 2 - Non-Polluting Solution Constructor (2-opt & Candidate 3-opt)

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:275-385`

**Interfaces:**
- Produces:
  - Clean `two_opt` and `three_opt` merging that returns `(Vec<Clause>, Vec<Vec<i32>>)` without injecting synthetic intermediate blocking clauses into `CaDiCaL`.

- [ ] **Step 1: Refactor `two_opt` in `src/hcp_solver.rs` to be a pure solution constructor**

Ensure that:
1. `merge_cycles` (2-opt) and `merge_three_cycles` (3-opt) run on the active cycles to find a full Hamiltonian Cycle.
2. If `cycles.len() == 1`, return immediately with Hamiltonian Cycle.
3. If `cycles.len() > 1`, return ONLY the standard ASP blocking clauses of the original unmerged/active cycles:
```rust
fn two_opt(
    sol_cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    block_method: i32,
    balanced: i32,
    opt: i32,
    three_opt: i32,
) -> (Vec<Clause>, Vec<Vec<i32>>) {
    let mut cycles = sol_cycles.to_vec();
    let mut merged = true;
    let mut cache_vertex: HashSet<usize> = HashSet::new();
    let mut active_cycles_number: Vec<usize> = (0..cycles.len()).collect();

    while merged {
        let (_dummy_clauses, new_merged, merged_numbers, new_cycle) =
            merge_cycles(&cycles, encoder, g, block_method, balanced, &mut cache_vertex, &active_cycles_number, opt);
        merged = new_merged;

        if merged {
            cycles.push(new_cycle);
            active_cycles_number.swap_remove(merged_numbers.1);
            active_cycles_number.swap_remove(merged_numbers.0);
            active_cycles_number.push(cycles.len() - 1);
        }

        // Check 3-opt candidate merge if 2-opt cannot merge further
        if !merged && three_opt == 1 && active_cycles_number.len() >= 3 {
            let (_three_clauses, three_merged, three_indices, three_cycle) =
                merge_three_cycles(&cycles, encoder, g, block_method, balanced, &active_cycles_number);
            if three_merged {
                cycles.push(three_cycle);
                let (ia, ib, ic) = three_indices;
                let mut remove_indices = [ia, ib, ic];
                remove_indices.sort_unstable_by(|a, b| b.cmp(a));
                for &idx in &remove_indices {
                    active_cycles_number.swap_remove(idx);
                }
                active_cycles_number.push(cycles.len() - 1);
                merged = true;
                cache_vertex.clear();
                continue;
            }
        }

        if active_cycles_number.len() == 1 || !merged {
            break;
        }
    }

    let active_cycles = get_active_cycles(&cycles, &active_cycles_number);
    let block_clauses = get_blocking_clauses(&active_cycles, encoder, g, block_method, balanced);

    (block_clauses, active_cycles)
}
```

- [ ] **Step 2: Build and verify**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo build --release`
Expected: Build success.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "refactor: convert 2-opt and 3-opt merging to non-polluting solution constructor"
```

---

### Task 3: Pillar 3 - Strong Minimal Subcycle Cuts & Clean CEGAR Loop

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:60-230`

- [ ] **Step 1: Clean up CEGAR loop in `src/hcp_solver.rs`**

1. Remove any unneeded MTZ stall injection or CEGAR fallback cutting that alters the CaDiCaL search space.
2. For cycles with $|C| \le 4$, strengthen the cut by including both the Cut-set and the exclusion clause.
3. For cycles with $|C| > 4$, generate standard ASP cuts.

```rust
// In hcp_solver.rs cegar() loop:
let (block_clauses, remaining_cycles) = if opt == 0 {
    (get_blocking_clauses(&sol_cycles, encoder, &g, block_method, balanced), sol_cycles.clone())
} else {
    let (clauses, cycles) = two_opt(&sol_cycles, encoder, &g, block_method, balanced, opt, three_opt);
    if cycles.len() == 1 {
        let flat: Vec<i32> = cycles.into_iter().flatten().collect();
        let line = flat.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
        println!();
        println!("hamiltonian cycle found by 2-opt/3-opt");
        println!("solution: ");
        println!("{}\n", line);
        println!("s SATISFIABLE");
        return (count, clause_count);
    }
    (clauses, cycles)
};

let mut cnf = Cnf::new();
cnf.extend(block_clauses);
clause_count += cnf.len() as i32;
let _ = solver.add_cnf(cnf);
count += 1;
```

- [ ] **Step 2: Build release binary**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo build --release`
Expected: Build finished with 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement strong minimal subcycle cuts and clean CEGAR loop"
```

---

### Task 4: Comprehensive Verification & Benchmarking

**Files:**
- Verify: `src/cegar-fix/target/release/cegar-fix`
- Test: All 24 previously timed-out graphs (`graph161`, `graph178`, `graph248`, `graph313`, `graph348`...)
- Test: Hard graphs (`graph45`, `graph132`, `graph339`)

- [ ] **Step 1: Run regression test on all 24 previous timeout graphs**

Run test script checking that 24/24 graphs solve within 60 seconds each.
Expected: 24/24 SOLVED, 0 TIMEOUT.

- [ ] **Step 2: Run verification on hard graphs (`graph45.col`, `graph132.col`, `graph339.col`)**

Run:
```bash
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph45.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph132.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
```
Expected: `s SATISFIABLE` in < 5 seconds.

- [ ] **Step 3: Update benchmark runner `run_fhcpcs_sota.sh` and commit**

```bash
git add run_fhcpcs_sota.sh
git commit -m "chore: update benchmark script to use three-pillar architecture"
```
