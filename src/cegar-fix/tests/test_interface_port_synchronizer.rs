use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::interface_port_synchronizer::InterfacePortSynchronizer;
use rustsat::clause;
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;
use std::collections::HashSet;

/// Constructs a graph containing a 16-node ladder gadget module (nodes 1..=16)
/// connected to an external 8-node module (nodes 17..=24).
fn build_ladder_gadget_graph() -> Graph {
    let mut g = Graph::new();

    // Module 0: 16-node ladder gadget
    // Top rail
    for i in 1..8 {
        g.add_edge(i, i + 1);
    }
    // Bottom rail
    for i in 9..16 {
        g.add_edge(i, i + 1);
    }
    // Rungs
    for i in 1..=8 {
        g.add_edge(i, i + 8);
    }
    // Diagonals inside 2x2 blocks
    g.add_edge(1, 10);
    g.add_edge(2, 9);

    g.add_edge(3, 12);
    g.add_edge(4, 11);

    g.add_edge(5, 14);
    g.add_edge(6, 13);

    g.add_edge(7, 16);
    g.add_edge(8, 15);

    // Diagonals between 2x2 blocks to create shared neighbors and strong components across the full 16-node gadget
    g.add_edge(2, 11);
    g.add_edge(3, 10);

    g.add_edge(4, 13);
    g.add_edge(5, 12);

    g.add_edge(6, 15);
    g.add_edge(7, 14);

    // Module 1: 8-node external module (17..=24)
    // Top rail (17..20), Bottom rail (21..24)
    for i in 17..20 {
        g.add_edge(i, i + 1);
    }
    for i in 21..24 {
        g.add_edge(i, i + 1);
    }
    for i in 17..=20 {
        g.add_edge(i, i + 4);
    }
    g.add_edge(17, 22);
    g.add_edge(18, 21);
    g.add_edge(18, 23);
    g.add_edge(19, 22);
    g.add_edge(19, 24);
    g.add_edge(20, 23);

    // Inter-module bridges: only via ports 1 and 8 of Module 0
    g.add_edge(8, 17);
    g.add_edge(20, 1);

    g
}

#[test]
fn test_extract_dual_paths_on_ladder_gadget() {
    let g = build_ladder_gadget_graph();
    let dual_paths = InterfacePortSynchronizer::extract_gadget_dual_paths(&g, 25);

    assert!(
        !dual_paths.is_empty(),
        "Should extract at least one gadget dual path"
    );

    // Find the 16-node gadget module
    let gadget = dual_paths
        .iter()
        .find(|d| d.vertices.len() == 16)
        .expect("Must find 16-node ladder gadget dual path");

    assert_eq!(gadget.ports, [1, 8], "Interface ports must be [1, 8]");
    assert_eq!(
        gadget.true_path_edges.len(),
        15,
        "True path must have 15 edges spanning 16 vertices"
    );
    assert_eq!(
        gadget.false_path_edges.len(),
        15,
        "False path must have 15 edges spanning 16 vertices"
    );
    assert_ne!(
        gadget.true_path_edges, gadget.false_path_edges,
        "True and False paths must be distinct"
    );

    // Verify both paths start at port A (1) and end at port B (8)
    assert_eq!(gadget.true_path_edges.first().unwrap().0, 1);
    assert_eq!(gadget.true_path_edges.last().unwrap().1, 8);
    assert_eq!(gadget.false_path_edges.first().unwrap().0, 1);
    assert_eq!(gadget.false_path_edges.last().unwrap().1, 8);

    // Verify all 16 vertices are visited in each path
    let mut visited_t = HashSet::new();
    visited_t.insert(gadget.true_path_edges[0].0);
    for &(_, v) in &gadget.true_path_edges {
        visited_t.insert(v);
    }
    assert_eq!(visited_t.len(), 16, "True path must visit all 16 vertices");

    let mut visited_f = HashSet::new();
    visited_f.insert(gadget.false_path_edges[0].0);
    for &(_, v) in &gadget.false_path_edges {
        visited_f.insert(v);
    }
    assert_eq!(visited_f.len(), 16, "False path must visit all 16 vertices");
}

#[test]
fn test_encode_interface_port_synchronization() {
    let g = build_ladder_gadget_graph();
    let dual_paths = InterfacePortSynchronizer::extract_gadget_dual_paths(&g, 25);
    assert!(!dual_paths.is_empty());

    let gadget0 = dual_paths
        .iter()
        .find(|d| d.vertices.len() == 16)
        .expect("Must find 16-node ladder gadget");

    let gadget1 = dual_paths
        .iter()
        .find(|d| d.vertices.len() == 8)
        .expect("Must find 8-node module");

    let true_edge_set: HashSet<(i32, i32)> = gadget0.true_path_edges.iter().copied().collect();
    let false_edge_set: HashSet<(i32, i32)> = gadget0.false_path_edges.iter().copied().collect();

    // Identify difference edges
    let t_minus_f: Vec<(i32, i32)> = true_edge_set
        .difference(&false_edge_set)
        .copied()
        .collect();
    let f_minus_t: Vec<(i32, i32)> = false_edge_set
        .difference(&true_edge_set)
        .copied()
        .collect();

    assert!(!t_minus_f.is_empty(), "T \\ F must not be empty");
    assert!(!f_minus_t.is_empty(), "F \\ T must not be empty");

    // PART A: Activating T_k edges must be SAT and imply F_k \\ T_k edges are FALSE
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
        InterfacePortSynchronizer::encode_interface_port_synchronization(
            &dual_paths,
            &mut encoder,
            &mut cnf,
        );

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Force all edges of T_k in Module 0
        for &e in &gadget0.true_path_edges {
            let _ = solver.add_clause(clause![encoder.graph_lit_map[&e]]);
        }

        // Force all edges of T_k in Module 1
        for &e in &gadget1.true_path_edges {
            let _ = solver.add_clause(clause![encoder.graph_lit_map[&e]]);
        }

        // Force external bridges
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 17)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(20, 1)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(
            res,
            SolverResult::Sat,
            "Valid tour using True path must be SAT"
        );
    }

    // PART B: Activating F_k edges must be SAT and imply T_k \\ F_k edges are FALSE
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
        InterfacePortSynchronizer::encode_interface_port_synchronization(
            &dual_paths,
            &mut encoder,
            &mut cnf,
        );

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Force all edges of F_k in Module 0
        for &e in &gadget0.false_path_edges {
            let _ = solver.add_clause(clause![encoder.graph_lit_map[&e]]);
        }

        // Force all edges of T_k in Module 1
        for &e in &gadget1.true_path_edges {
            let _ = solver.add_clause(clause![encoder.graph_lit_map[&e]]);
        }

        // Force external bridges
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 17)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(20, 1)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(
            res,
            SolverResult::Sat,
            "Valid tour using False path must be SAT"
        );
    }

    // PART C: Conflicting choice (activating an edge from T \ F AND an edge from F \ T) must be UNSAT
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
        InterfacePortSynchronizer::encode_interface_port_synchronization(
            &dual_paths,
            &mut encoder,
            &mut cnf,
        );

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        let e_t = t_minus_f[0];
        let e_f = f_minus_t[0];

        let _ = solver.add_clause(clause![encoder.graph_lit_map[&e_t]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&e_f]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(
            res,
            SolverResult::Unsat,
            "Activating both T and F conflicting edges must be UNSAT"
        );
    }
}

#[test]
fn test_perimeter_loop_forbidden() {
    let g = build_ladder_gadget_graph();
    let dual_paths = InterfacePortSynchronizer::extract_gadget_dual_paths(&g, 25);
    assert!(!dual_paths.is_empty());

    let mut encoder = Encoder::new();
    let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    InterfacePortSynchronizer::encode_interface_port_synchronization(
        &dual_paths,
        &mut encoder,
        &mut cnf,
    );

    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    // Attempt to force the isolated perimeter cycle:
    // 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 16 -> 15 -> 14 -> 13 -> 12 -> 11 -> 10 -> 9 -> 1
    // The channeling constraints place unit clauses (!e) on non-path edges like (8, 16) or (9, 1).
    let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 16)]]);
    let _ = solver.add_clause(clause![encoder.graph_lit_map[&(9, 1)]]);

    let res = solver.solve().expect("solve failed");
    assert_eq!(
        res,
        SolverResult::Unsat,
        "Isolated perimeter cycle using non-path edges must be UNSAT"
    );
}
