use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use rustsat::types::Lit;

pub trait LitMap {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit>;
}

impl LitMap for HashMap<(i32, i32), Lit> {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit> {
        self.get(&(u, v)).copied()
    }
}

impl LitMap for BTreeMap<(i32, i32), Lit> {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit> {
        self.get(&(u, v)).copied()
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmpiricalBackboneTracker {
    pub history_window: usize,
    pub edge_history: VecDeque<HashSet<(i32, i32)>>,
    pub total_rounds_recorded: usize,
}

impl EmpiricalBackboneTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            history_window: if window_size == 0 { 10 } else { window_size },
            edge_history: VecDeque::new(),
            total_rounds_recorded: 0,
        }
    }

    pub fn record_solution_edges(&mut self, cycles: &[Vec<i32>]) {
        let mut round_edges = HashSet::new();
        for cycle in cycles {
            let n = cycle.len();
            if n < 2 {
                continue;
            }
            for i in 0..n {
                let u = cycle[i];
                let v = cycle[(i + 1) % n];
                if u != v {
                    let min_v = u.min(v);
                    let max_v = u.max(v);
                    round_edges.insert((min_v, max_v));
                }
            }
        }

        self.edge_history.push_back(round_edges);
        self.total_rounds_recorded += 1;

        let max_window = if self.history_window == 0 { 10 } else { self.history_window };
        while self.edge_history.len() > max_window {
            self.edge_history.pop_front();
        }
    }

    pub fn get_frequent_backbone_edges(&self, threshold: f64) -> HashSet<(i32, i32)> {
        if self.edge_history.is_empty() {
            return HashSet::new();
        }

        let mut edge_counts: HashMap<(i32, i32), usize> = HashMap::new();
        for round_edges in &self.edge_history {
            for &edge in round_edges {
                *edge_counts.entry(edge).or_insert(0) += 1;
            }
        }

        let denominator = self.edge_history.len() as f64;
        let mut result = HashSet::new();
        for (edge, count) in edge_counts {
            let freq = (count as f64) / denominator;
            if freq >= threshold {
                result.insert(edge);
            }
        }

        result
    }
}

pub struct EmpiricalBackboneCutter;

impl EmpiricalBackboneCutter {
    pub fn generate_comprehensive_sec_clauses<M: LitMap + ?Sized>(
        cycles: &[Vec<i32>],
        giant_threshold: usize,
        lit_map: &M,
    ) -> Vec<Vec<Lit>> {
        let mut clauses = Vec::new();

        for cycle in cycles {
            let k = cycle.len();
            if k < 3 || k >= giant_threshold {
                continue;
            }

            // Forward direction: cycle[i] -> cycle[(i + 1) % k]
            let mut fwd_clause = Vec::with_capacity(k);
            let mut fwd_ok = true;
            for i in 0..k {
                let u = cycle[i];
                let v = cycle[(i + 1) % k];
                if let Some(lit) = lit_map.get_lit(u, v) {
                    fwd_clause.push(!lit);
                } else {
                    fwd_ok = false;
                    break;
                }
            }
            if fwd_ok {
                clauses.push(fwd_clause);
            }

            // Reverse direction: cycle[(i + 1) % k] -> cycle[i]
            let mut rev_clause = Vec::with_capacity(k);
            let mut rev_ok = true;
            for i in 0..k {
                let u = cycle[(i + 1) % k];
                let v = cycle[i];
                if let Some(lit) = lit_map.get_lit(u, v) {
                    rev_clause.push(!lit);
                } else {
                    rev_ok = false;
                    break;
                }
            }
            if rev_ok {
                clauses.push(rev_clause);
            }
        }

        clauses
    }
}
