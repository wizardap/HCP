use std::collections::HashSet;
use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::module_state_cnf_encoder::ModuleStateCnfEncoder;
use rustsat::clause;
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;

fn build_test_graph() -> (Graph, Vec<i32>, Vec<i32>, Vec<i32>) {
    let mut g = Graph::new();

    // Module 0: 6 nodes (1..=6)
    // Top: 1-2-3, Bottom: 4-5-6
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    // Rungs
    g.add_edge(1, 4);
    g.add_edge(2, 5);
    g.add_edge(3, 6);
    // Diagonals
    g.add_edge(1, 5);
    g.add_edge(2, 4);
    g.add_edge(2, 6);
    g.add_edge(3, 5);

    // External module 1: 7, 8
    g.add_edge(7, 8);
    // Bridges between Module 0 and Module 1
    // Port 1 is node 1, Port 2 is node 6
    g.add_edge(6, 7);
    g.add_edge(8, 1);

    let mod_vertices = vec![1, 2, 3, 4, 5, 6];
    // True path: 1 -> 2 -> 4 -> 5 -> 3 -> 6 (spans all 6 vertices from 1 to 6)
    let t_path = vec![1, 2, 4, 5, 3, 6];
    // False path: 1 -> 5 -> 2 -> 3 -> 5... wait, 1 -> 4 -> 2 -> 5 -> 3 -> 6 (also spans all 6)
    let f_path = vec![1, 4, 2, 5, 3, 6];

    (g, mod_vertices, t_path, f_path)
}

#[test]
fn test_module_state_cnf_encoder_forcing() {
    let (g, mod_vertices, t_path, f_path) = build_test_graph();

    let mut encoder = Encoder::new();
    let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let added_clauses = ModuleStateCnfEncoder::encode_module_dual_state(
        &mod_vertices,
        &t_path,
        &f_path,
        &g,
        &mut encoder,
        &mut cnf,
    );

    assert!(added_clauses > 0, "Must add equivalence clauses");

    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    // Force external bridges
    let _ = solver.add_clause(clause![encoder.graph_lit_map[&(6, 7)]]);
    let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 8)]]);
    let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 1)]]);

    let res = solver.solve().expect("solve failed");
    assert_eq!(res, SolverResult::Sat, "Valid Hamiltonian cycle using module path must be SAT");
}
