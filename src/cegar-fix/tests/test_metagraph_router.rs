use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::metagraph_router::{GadgetModule, MetagraphRouter};
use rustsat::clause;
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;
use std::collections::HashSet;

#[test]
fn test_detect_gadget_modules() {
    let mut g = Graph::new();

    // Module 0: vertices 1, 2, 3
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Module 1: vertices 4, 5, 6
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Module 2: vertices 7, 8, 9
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 7);

    // Bridge / boundary edges connecting the modules in a ring
    g.add_edge(3, 4);
    g.add_edge(6, 7);
    g.add_edge(9, 1);

    let modules = MetagraphRouter::detect_gadget_modules(&g);
    assert_eq!(modules.len(), 3, "Expected exactly 3 gadget modules");

    let mut all_vertices = HashSet::new();
    for module in &modules {
        assert!(!module.vertices.is_empty());
        for &v in &module.vertices {
            assert!(all_vertices.insert(v), "Duplicate vertex in module partitioning");
        }
        // Each module should have boundary edges connecting to outside vertices
        assert!(!module.boundary_edges.is_empty(), "Module {} should have boundary edges", module.id);
        for &(u, v) in &module.boundary_edges {
            assert!(module.vertices.contains(&u), "Boundary edge origin must be in module");
            assert!(!module.vertices.contains(&v), "Boundary edge target must not be in module");
        }
    }
    assert_eq!(all_vertices.len(), 9);
}

#[test]
fn test_encode_supernode_mtz() {
    let mut g = Graph::new();

    // 3 Modules forming a ring:
    // Module 0: 1, 2, 3
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Module 1: 4, 5, 6
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Module 2: 7, 8, 9
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 7);

    // Inter-module ring edges:
    // 3 -> 4, 4 -> 3
    g.add_edge(3, 4);
    // 6 -> 7, 7 -> 6
    g.add_edge(6, 7);
    // 9 -> 1, 1 -> 9
    g.add_edge(9, 1);

    let modules = MetagraphRouter::detect_gadget_modules(&g);
    assert_eq!(modules.len(), 3);

    // PART A: Assert disconnected 3-subcycle state (each module in its own internal 3-cycle)
    // MTZ on supernodes must make this state UNSAT
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        let initial_clauses = cnf.len();
        MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);
        assert!(cnf.len() > initial_clauses, "Supernode MTZ should add clauses to CNF");

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Force internal cycles in all 3 modules:
        // Mod 0: 1->2->3->1
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(1, 2)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(2, 3)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(3, 1)]]);

        // Mod 1: 4->5->6->4
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(4, 5)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(5, 6)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(6, 4)]]);

        // Mod 2: 7->8->9->7
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 8)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 9)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(9, 7)]]);

        // Force boundary edges = FALSE
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(3, 4)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(4, 3)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(6, 7)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(7, 6)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(9, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(1, 9)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Unsat, "Disconnected module subcycles must be UNSAT under supernode MTZ");
    }

    // PART B: Assert connected valid 9-cycle traversing all 3 modules
    // 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9 -> 1
    // Must be SAT
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Force the single 9-cycle:
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(1, 2)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(2, 3)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(3, 4)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(4, 5)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(5, 6)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(6, 7)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 8)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 9)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(9, 1)]]);

        // Force non-cycle edges to false
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(3, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(6, 4)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(9, 7)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Sat, "Connected 9-cycle traversing all modules must be SAT under supernode MTZ");
    }
}

#[test]
fn test_small_k_supernode_mtz_no_op() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    let mod0 = GadgetModule {
        id: 0,
        vertices: vec![1, 2],
        boundary_edges: vec![(2, 3)],
    };
    let mod1 = GadgetModule {
        id: 1,
        vertices: vec![3],
        boundary_edges: vec![(3, 1)],
    };

    let modules = vec![mod0, mod1]; // K = 2 <= 2
    let mut encoder = Encoder::new();
    let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    let count_before = cnf.len();

    MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);
    assert_eq!(cnf.len(), count_before, "K <= 2 must be a no-op");
}

#[test]
fn test_empty_graph_and_single_module() {
    let empty_g = Graph::new();
    let empty_modules = MetagraphRouter::detect_gadget_modules(&empty_g);
    assert!(empty_modules.is_empty());

    let mut single_g = Graph::new();
    single_g.add_edge(1, 2);
    single_g.add_edge(2, 3);
    single_g.add_edge(3, 1);

    let single_modules = MetagraphRouter::detect_gadget_modules(&single_g);
    assert_eq!(single_modules.len(), 1);
    assert_eq!(single_modules[0].vertices, vec![1, 2, 3]);
    assert!(single_modules[0].boundary_edges.is_empty());
}

#[test]
fn test_encode_supernode_mtz_four_modules_subcycles() {
    let mut g = Graph::new();

    // 4 Modules:
    // Module 0: 1, 2, 3
    // Module 1: 4, 5, 6
    // Module 2: 7, 8, 9
    // Module 3: 10, 11, 12
    for m in 0..4 {
        let base = m * 3 + 1;
        g.add_edge(base, base + 1);
        g.add_edge(base + 1, base + 2);
        g.add_edge(base + 2, base);
    }

    // Connect into 4-ring: (3, 4), (6, 7), (9, 10), (12, 1)
    g.add_edge(3, 4);
    g.add_edge(6, 7);
    g.add_edge(9, 10);
    g.add_edge(12, 1);

    // Also add short-cut bridges (6, 1) and (12, 7) creating two disjoint 2-module loops:
    // Loop A: Module 0 <-> Module 1
    // Loop B: Module 2 <-> Module 3
    g.add_edge(6, 1);
    g.add_edge(12, 7);

    let modules = MetagraphRouter::detect_gadget_modules(&g);
    assert_eq!(modules.len(), 4);

    // Subcycle test: Loop A (Mod 0 - Mod 1) and Loop B (Mod 2 - Mod 3) active simultaneously
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Force Loop A: 1->2->3->4->5->6->1
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(1, 2)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(2, 3)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(3, 4)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(4, 5)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(5, 6)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(6, 1)]]);

        // Force Loop B: 7->8->9->10->11->12->7
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 8)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 9)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(9, 10)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(10, 11)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(11, 12)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(12, 7)]]);

        // Deactivate cross bridges between Loop A and Loop B: (6, 7) and (12, 1)
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(6, 7)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(7, 6)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(12, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(1, 12)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Unsat, "Disconnected 2-loop state (Loop A & Loop B) must be UNSAT under MTZ");
    }

    // Valid 12-cycle test: 1->2->3->4->5->6->7->8->9->10->11->12->1
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(1, 2)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(2, 3)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(3, 4)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(4, 5)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(5, 6)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(6, 7)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 8)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(8, 9)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(9, 10)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(10, 11)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(11, 12)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(12, 1)]]);

        // False for shortcuts and internal closes
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(6, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(12, 7)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(3, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(6, 4)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(9, 7)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(12, 10)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Sat, "Valid 12-cycle traversing all 4 modules must be SAT");
    }
}
