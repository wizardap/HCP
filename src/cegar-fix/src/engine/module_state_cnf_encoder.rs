use std::collections::HashSet;
use rustsat::clause;
use rustsat::instances::Cnf;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct ModuleStateCnfEncoder;

impl ModuleStateCnfEncoder {
    /// Encodes a state choice literal b_k for a module:
    /// - b_k = 1 forces all edges in T_k \ F_k and forbids F_k \ T_k
    /// - b_k = 0 forces all edges in F_k \ T_k and forbids T_k \ F_k
    /// - Edges in T_k \cap F_k are forced active (shared backbone)
    /// - Edges internal to the module not in T_k \cup F_k are forbidden
    ///
    /// This completely eliminates all spurious intra-module subcycles (e.g. 16-cycles).
    pub fn encode_module_dual_state(
        mod_vertices: &[i32],
        t_path: &[i32],
        f_path: &[i32],
        g: &Graph,
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    ) -> usize {
        if mod_vertices.len() < 2 || t_path.is_empty() || f_path.is_empty() {
            return 0;
        }

        let mod_set: HashSet<i32> = mod_vertices.iter().copied().collect();

        // 1. Allocate boolean choice literal b_k for this module
        let b_k = encoder.instance.new_lit();

        let true_set: HashSet<(i32, i32)> = t_path
            .windows(2)
            .map(|w| if w[0] < w[1] { (w[0], w[1]) } else { (w[1], w[0]) })
            .collect();

        let false_set: HashSet<(i32, i32)> = f_path
            .windows(2)
            .map(|w| if w[0] < w[1] { (w[0], w[1]) } else { (w[1], w[0]) })
            .collect();

        // 2. Identify all unique undirected internal edges in G[M_k]
        let mut internal_undirected_edges: HashSet<(i32, i32)> = HashSet::new();
        for &u in mod_vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if mod_set.contains(&v) && u < v {
                        internal_undirected_edges.insert((u, v));
                    }
                }
            }
        }

        let mut added_clauses = 0;

        // 3. Channeling clauses for all undirected internal edges
        for (u, v) in internal_undirected_edges {
            let l_uv = encoder.graph_lit_map.get(&(u, v)).copied();
            let l_vu = encoder.graph_lit_map.get(&(v, u)).copied();

            let in_t = true_set.contains(&(u, v));
            let in_f = false_set.contains(&(u, v));

            match (l_uv, l_vu) {
                (Some(fwd), Some(rev)) => {
                    if in_t && !in_f {
                        // b_k => (fwd \/ rev)
                        cnf.add_clause(clause![!b_k, fwd, rev]);
                        // !b_k => (!fwd /\ !rev)
                        cnf.add_clause(clause![b_k, !fwd]);
                        cnf.add_clause(clause![b_k, !rev]);
                        added_clauses += 3;
                    } else if in_f && !in_t {
                        // !b_k => (fwd \/ rev)
                        cnf.add_clause(clause![b_k, fwd, rev]);
                        // b_k => (!fwd /\ !rev)
                        cnf.add_clause(clause![!b_k, !fwd]);
                        cnf.add_clause(clause![!b_k, !rev]);
                        added_clauses += 3;
                    } else if in_t && in_f {
                        // Shared backbone edge: must be used in one direction
                        cnf.add_clause(clause![fwd, rev]);
                        added_clauses += 1;
                    } else {
                        // Forbidden non-path edge: neither direction can be used
                        cnf.add_clause(clause![!fwd]);
                        cnf.add_clause(clause![!rev]);
                        added_clauses += 2;
                    }
                }
                (Some(lit), None) | (None, Some(lit)) => {
                    if in_t && !in_f {
                        cnf.add_clause(clause![!b_k, lit]);
                        cnf.add_clause(clause![b_k, !lit]);
                        added_clauses += 2;
                    } else if in_f && !in_t {
                        cnf.add_clause(clause![b_k, lit]);
                        cnf.add_clause(clause![!b_k, !lit]);
                        added_clauses += 2;
                    } else if in_t && in_f {
                        cnf.add_clause(clause![lit]);
                        added_clauses += 1;
                    } else {
                        cnf.add_clause(clause![!lit]);
                        added_clauses += 1;
                    }
                }
                (None, None) => {}
            }
        }

        added_clauses
    }
}
