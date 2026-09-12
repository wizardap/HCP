use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use cegar_fix::port_corridor_lns::PortCorridorLns;
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;
use rustsat::types::TernaryVal;

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

    // Connect block 0 to 1, 1 to 2, 2 to 3, but ALSO allow a shortcut cycle between 1 and 2
    g.add_edge(1, 2); // 0 -> 1
    g.add_edge(3, 4); // 1 -> 2
    g.add_edge(5, 6); // 2 -> 3
    g.add_edge(3, 5); // 1 -> 2 cross
    g.add_edge(2, 4); // 1 -> 2 cross back (forms 2-cycle between block 1 & 2)

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let mut local_solver = CaDiCaL::default();
    for cl in base_cnf.iter() {
        let _ = local_solver.add_clause(cl.clone());
    }

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

    let mut next_free_var = encoder.instance.n_vars() as i32 + 10;
    let (_order_vars, n_clauses) = PortCorridorLns::inject_unary_mtz_ordering(
        &mut local_solver,
        &corridor,
        &encoder,
        &g,
        &node_to_port,
        &port_to_node,
        &mut next_free_var,
    );

    assert!(n_clauses > 0, "Must inject MTZ clauses");

    let res = local_solver.solve();
    assert_eq!(res.unwrap(), SolverResult::Sat);
    let sol = local_solver.full_solution().unwrap();

    // Verify that the solution is a single 4-block path, NOT a shortcut + 2-block cycle!
    let mut active_edges = HashSet::new();
    for (&(u, v), &lit) in &encoder.graph_lit_map {
        if sol.lit_value(lit) == TernaryVal::True {
            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                if p1.block != p2.block {
                    active_edges.insert(min_max(u, v));
                }
            }
        }
    }

    let (cycs, _) = AlternatingPortEngine::get_cycles(&active_edges, &node_to_port, &port_to_node, n_blocks);
    // With MTZ, no disconnected cycles can exist inside the corridor!
    for c in &cycs {
        assert!(c.len() > 4, "No 2-block cycles permitted by MTZ");
    }
}
