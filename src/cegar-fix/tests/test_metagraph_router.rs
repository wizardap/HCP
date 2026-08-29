use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::metagraph_router::{ChannelModule, GadgetModule, MetagraphRouter};
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

#[test]
fn test_graph479_metagraph() {
    use cegar_fix::file_operations::input_to_graph;
    use cegar_fix::contraction::Degree2Contractor;
    use std::path::Path;

    let path = "/home/ubuntu/HCP/FHCPCS-col/graph479.col";
    if Path::new(path).exists() {
        let mut g = input_to_graph(path);
        g.prune_degree2_triangles();
        let (contracted_g, _) = Degree2Contractor::contract(&g);
        let mods = MetagraphRouter::detect_gadget_modules(&contracted_g);
        assert!(!mods.is_empty(), "Modules should be detected");
        let total_verts: usize = mods.iter().map(|m| m.vertices.len()).sum();
        assert_eq!(total_verts, contracted_g.adjacency_list.len(), "Modules must partition all vertices");
    }
}

#[test]
fn test_detect_dual_channels() {
    let mut g = Graph::new();

    // Create a 24-vertex gadget module (1..=24) with dense internal triangles
    for i in 1..=24 {
        let next = if i == 24 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }
    for i in 1..=24 {
        let next2 = if i >= 23 { i + 2 - 24 } else { i + 2 };
        g.add_edge(i, next2);
    }
    // Also create a small 3-vertex module (25..=27)
    g.add_edge(25, 26);
    g.add_edge(26, 27);
    g.add_edge(27, 25);

    // Connect them with bridge edges
    g.add_edge(24, 25);
    g.add_edge(27, 1);

    let channels = MetagraphRouter::detect_dual_channels(&g);

    // Module 0 had 24 vertices (> 12) -> should be split into 2 channel modules of 12 vertices each
    // Module 1 had 3 vertices (<= 12) -> should remain 1 channel module of 3 vertices
    // Total channels = 3
    assert_eq!(channels.len(), 3, "Expected 3 channel modules (2 for large module, 1 for small module)");

    let c0 = &channels[0];
    let c1 = &channels[1];
    let c2 = &channels[2];

    assert_eq!(c0.parent_gadget_id, 0);
    assert_eq!(c0.channel_idx, 0);
    assert_eq!(c0.vertices.len(), 12);

    assert_eq!(c1.parent_gadget_id, 0);
    assert_eq!(c1.channel_idx, 1);
    assert_eq!(c1.vertices.len(), 12);

    assert_eq!(c2.parent_gadget_id, 1);
    assert_eq!(c2.channel_idx, 0);
    assert_eq!(c2.vertices.len(), 3);

    // Check vertex partitioning (all vertices 1..=27 disjoint and present)
    let mut all_v = HashSet::new();
    for ch in &channels {
        for &v in &ch.vertices {
            assert!(all_v.insert(v), "Duplicate vertex {} across channels", v);
        }
        assert!(!ch.boundary_edges.is_empty(), "Channel {} should have boundary edges", ch.id);
        for &(u, v) in &ch.boundary_edges {
            assert!(ch.vertices.contains(&u), "Boundary edge source {} must be in channel {}", u, ch.id);
            assert!(!ch.vertices.contains(&v), "Boundary edge target {} must not be in channel {}", v, ch.id);
        }
    }
    assert_eq!(all_v.len(), 27);
}

#[test]
fn test_encode_dual_channel_mtz_soundness() {
    let mut g = Graph::new();

    // 3 Modules of 14 vertices each (> 12), so each splits into 2 subchannels of 7 vertices
    // Total = 6 channels: C0 (1..7), C1 (8..14), C2 (15..21), C3 (22..28), C4 (29..35), C5 (36..42)
    for m in 0..3 {
        let base = m * 14;
        // Channel A vertices: base+1..=base+7
        for i in 1..=6 {
            g.add_edge(base + i, base + i + 1);
        }
        for i in (1..=5).step_by(2) {
            g.add_edge(base + i, base + i + 2);
        }

        // Channel B vertices: base+8..=base+14
        for i in 8..=13 {
            g.add_edge(base + i, base + i + 1);
        }
        for i in (8..=12).step_by(2) {
            g.add_edge(base + i, base + i + 2);
        }

        // Internal intra-module bridge between Channel A and Channel B
        g.add_edge(base + 7, base + 8);
        g.add_edge(base + 14, base + 1);
    }

    // Inter-module bridges:
    // C0 (7) -> C2 (15)
    g.add_edge(7, 15);
    // C2 (21) -> C4 (29)
    g.add_edge(21, 29);
    // C4 (35) -> C1 (8)
    g.add_edge(35, 8);
    // C1 (14) -> C3 (22)
    g.add_edge(14, 22);
    // C3 (28) -> C5 (36)
    g.add_edge(28, 36);
    // C5 (42) -> C0 (1)
    g.add_edge(42, 1);

    // Subcycle shortcut bridges:
    // C4 (35) -> C0 (1) to close Loop A: C0 -> C2 -> C4 -> C0
    g.add_edge(35, 1);
    // C5 (42) -> C1 (8) to close Loop B: C1 -> C3 -> C5 -> C1
    g.add_edge(42, 8);

    let channels = MetagraphRouter::detect_dual_channels(&g);
    assert_eq!(channels.len(), 6, "Expected 6 dual-channel submodules");

    // PART A: Assert disconnected 2-subcycle state (Loop A: C0->C2->C4->C0 and Loop B: C1->C3->C5->C1)
    // MTZ on dual channels MUST make this state UNSAT
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        let initial_clauses = cnf.len();
        MetagraphRouter::encode_dual_channel_mtz(&channels, &mut encoder, &mut cnf);
        assert!(cnf.len() > initial_clauses, "Dual channel MTZ should add clauses to CNF");

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Internal paths inside channels:
        for m in 0..3 {
            let base = m * 14;
            for i in 1..=6 {
                let _ = solver.add_clause(clause![encoder.graph_lit_map[&(base + i, base + i + 1)]]);
            }
            for i in 8..=13 {
                let _ = solver.add_clause(clause![encoder.graph_lit_map[&(base + i, base + i + 1)]]);
            }
        }

        // Force Loop A: 7 -> 15, 21 -> 29, 35 -> 1
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 15)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(21, 29)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(35, 1)]]);

        // Force Loop B: 14 -> 22, 28 -> 36, 42 -> 8
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(14, 22)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(28, 36)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(42, 8)]]);

        // Deactivate full 2-pass cross bridges
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(35, 8)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(8, 35)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(42, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(1, 42)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Unsat, "Disconnected dual-channel subcycles must be UNSAT under dual-channel MTZ");
    }

    // PART B: Assert connected valid 2-pass 42-vertex tour visiting all 6 channels
    // C0 (1..7) -> C2 (15..21) -> C4 (29..35) -> C1 (8..14) -> C3 (22..28) -> C5 (36..42) -> C0 (1)
    // Must be SAT
    {
        let mut encoder = Encoder::new();
        let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

        MetagraphRouter::encode_dual_channel_mtz(&channels, &mut encoder, &mut cnf);

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        // Internal paths inside channels:
        for m in 0..3 {
            let base = m * 14;
            for i in 1..=6 {
                let _ = solver.add_clause(clause![encoder.graph_lit_map[&(base + i, base + i + 1)]]);
            }
            for i in 8..=13 {
                let _ = solver.add_clause(clause![encoder.graph_lit_map[&(base + i, base + i + 1)]]);
            }
        }

        // Force 2-pass tour bridges:
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(7, 15)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(21, 29)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(35, 8)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(14, 22)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(28, 36)]]);
        let _ = solver.add_clause(clause![encoder.graph_lit_map[&(42, 1)]]);

        // Deactivate shortcut subcycle bridges
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(35, 1)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(1, 35)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(42, 8)]]);
        let _ = solver.add_clause(clause![!encoder.graph_lit_map[&(8, 42)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Sat, "Valid 2-pass tour visiting all dual channels must be SAT");
    }
}

#[test]
fn test_small_k_dual_channel_mtz_no_op() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    let ch0 = ChannelModule {
        id: 0,
        parent_gadget_id: 0,
        channel_idx: 0,
        vertices: vec![1, 2],
        boundary_edges: vec![(2, 3)],
    };
    let ch1 = ChannelModule {
        id: 1,
        parent_gadget_id: 0,
        channel_idx: 1,
        vertices: vec![3],
        boundary_edges: vec![(3, 1)],
    };

    let channels = vec![ch0, ch1]; // K = 2 <= 2
    let mut encoder = Encoder::new();
    let mut cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    let count_before = cnf.len();

    MetagraphRouter::encode_dual_channel_mtz(&channels, &mut encoder, &mut cnf);
    assert_eq!(cnf.len(), count_before, "K <= 2 must be a no-op for dual channel MTZ");
}







