use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use cegar_fix::port_corridor_lns::PortCorridorLns;
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat_cadical::CaDiCaL;

#[test]
fn test_unary_mtz_forbids_disconnected_subcycles() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 4 blocks: 0, 1, 2, 3
    for b in 0..4 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    // Full 4-block cycle edges: 0 -> 1 -> 2 -> 3 -> 0
    g.add_edge(1, 2); // block 0 to 1
    g.add_edge(3, 4); // block 1 to 2
    g.add_edge(5, 6); // block 2 to 3
    g.add_edge(7, 0); // block 3 to 0

    // Also shortcut forming two 2-block cycles: (0, 3) and (1, 2)
    g.add_edge(1, 6); // block 0 to 3 directly
    g.add_edge(3, 5); // block 1 to 2 cross
    g.add_edge(2, 4); // block 1 to 2 cross back

    let (_n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let mut sat_blocks = HashSet::new();
    sat_blocks.insert(1);
    sat_blocks.insert(2);
    let mut unfrozen_blocks = HashSet::new();
    for b in 0..4 { unfrozen_blocks.insert(b); }

    let corridor = cegar_fix::port_corridor_lns::PortSubpathCorridor {
        sat_blocks,
        giant_subpath_blocks: vec![0, 3],
        entry_port: Port { block: 0, end: 1 },
        exit_port: Port { block: 3, end: 0 },
        unfrozen_blocks,
    };

    // 1. Without MTZ: if we assume the 2-block cycle edge (2, 4), solver can satisfy with 2-block cycles!
    let mut solver_without_mtz = CaDiCaL::default();
    for cl in base_cnf.iter() {
        let _ = solver_without_mtz.add_clause(cl.clone());
    }
    let lit_2_4 = encoder.graph_lit_map[&min_max(2, 4)];
    let lit_3_5 = encoder.graph_lit_map[&min_max(3, 5)];
    let res_without = solver_without_mtz.solve_assumps(&[lit_2_4, lit_3_5]);
    assert_eq!(res_without.unwrap(), SolverResult::Sat, "Without MTZ, 2-block cycle is feasible");

    // 2. With MTZ: assuming the 2-block cycle edge (2, 4) MUST BE UNSAT because MTZ forbids internal cycle (1, 2)!
    let mut solver_with_mtz = CaDiCaL::default();
    for cl in base_cnf.iter() {
        let _ = solver_with_mtz.add_clause(cl.clone());
    }
    let mut next_free_var = encoder.instance.n_vars() as i32 + 10;
    let (_order_vars, n_clauses) = PortCorridorLns::inject_unary_mtz_ordering(
        &mut solver_with_mtz,
        &corridor,
        &encoder,
        &g,
        &node_to_port,
        &port_to_node,
        &mut next_free_var,
    );
    assert!(n_clauses > 0, "Must inject MTZ clauses");

    let res_with_subcycle = solver_with_mtz.solve_assumps(&[lit_2_4, lit_3_5]);
    assert_eq!(res_with_subcycle.unwrap(), SolverResult::Unsat, "With MTZ, 2-block internal cycle is strictly UNSAT!");
}
