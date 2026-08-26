use cegar_fix::global_demand_coordinator::GlobalDemandCoordinator;
use cegar_fix::graph::Graph;
use cegar_fix::macro_splicer::{patch_cycles_2opt, splice_macro_tour, verify_tour_on_raw_graph};
use cegar_fix::pinpointed_strip_solver::PinpointedStripSolver;
use cegar_fix::two_tier_decomposer::decompose_graph;
use std::collections::HashMap;

#[test]
fn test_verify_tour_on_raw_graph() {
    let mut g = Graph::new();
    // 4-cycle: 1 - 2 - 3 - 4 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Valid tours
    assert!(verify_tour_on_raw_graph(&[1, 2, 3, 4], &g));
    assert!(verify_tour_on_raw_graph(&[4, 3, 2, 1], &g));
    assert!(verify_tour_on_raw_graph(&[2, 3, 4, 1], &g));

    // Invalid: wrong length
    assert!(!verify_tour_on_raw_graph(&[1, 2, 3], &g));
    assert!(!verify_tour_on_raw_graph(&[1, 2, 3, 4, 1], &g));

    // Invalid: duplicate vertex
    assert!(!verify_tour_on_raw_graph(&[1, 2, 2, 4], &g));

    // Invalid: non-edge consecutive pair
    assert!(!verify_tour_on_raw_graph(&[1, 3, 2, 4], &g));
}

#[test]
fn test_patch_cycles_2opt_synthetic() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 2: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Cross edges: (1, 4) and (2, 5)
    // 2-opt can replace (1, 2) and (4, 5) with (1, 4) and (2, 5)
    g.add_edge(1, 4);
    g.add_edge(2, 5);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let patched = patch_cycles_2opt(cycles, &g);

    assert_eq!(patched.len(), 1, "Expected cycles to be merged into 1");
    assert_eq!(patched[0].len(), 6, "Expected merged tour of length 6");
    assert!(
        verify_tour_on_raw_graph(&patched[0], &g),
        "Patched tour must be valid on raw graph"
    );
}

#[test]
fn test_patch_cycles_2opt_three_cycles() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 2: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Cycle 3: 7 - 8 - 9 - 7
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 7);

    // Cross edges between C1 and C2: (1, 4) and (2, 5)
    g.add_edge(1, 4);
    g.add_edge(2, 5);

    // Cross edges between C2 and C3: (6, 7) and (4, 8)
    g.add_edge(6, 7);
    g.add_edge(4, 8);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    let patched = patch_cycles_2opt(cycles, &g);

    assert_eq!(patched.len(), 1, "Expected 3 cycles to merge into 1");
    assert_eq!(patched[0].len(), 9, "Expected merged tour of length 9");
    assert!(
        verify_tour_on_raw_graph(&patched[0], &g),
        "Merged tour must be valid on raw graph"
    );
}

#[test]
fn test_splice_macro_tour_synthetic() {
    let mut g = Graph::new();
    // Auxiliary hubs 10..35 form a cycle with chords to give degree >= 20
    for i in 10..35 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(35, 10);

    for i in 10..35 {
        for j in (i + 2)..=35 {
            if !(i == 10 && j == 35) {
                g.add_edge(i, j);
            }
        }
    }

    // 4 Hubs: 1, 2, 3, 4 connected to auxiliary hubs to have degree >= 20
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
        g.add_edge(3, i);
        g.add_edge(4, i);
    }

    // Hub-Hub edges: (1, 2) and (3, 4)
    g.add_edge(1, 2);
    g.add_edge(3, 4);

    // Strip 1: 101-102-103
    g.add_edge(101, 102);
    g.add_edge(102, 103);
    g.add_edge(1, 101);
    g.add_edge(3, 103);

    // Strip 2: 201-202-203
    g.add_edge(201, 202);
    g.add_edge(202, 203);
    g.add_edge(2, 201);
    g.add_edge(4, 203);

    let decomp = decompose_graph(&g);
    assert_eq!(decomp.strips.len(), 2);

    let mut coord = GlobalDemandCoordinator::new(&g, &decomp);
    let mut strip_solver = PinpointedStripSolver::new(&g, &decomp);
    let mut strip_paths = HashMap::new();
    let mut final_hh_edges = Vec::new();
    let mut final_strip_demands = HashMap::new();

    for _ in 0..100 {
        let assignment = coord
            .solve_assignment()
            .expect("Global coordinator must find an assignment");
        let (hh_edges, strip_demands) = assignment;
        let mut all_ok = true;
        strip_paths.clear();

        for (si, _) in decomp.strips.iter().enumerate() {
            let dem = strip_demands.get(&si).cloned().unwrap_or_default();
            let total_dem: usize = dem.values().sum();
            let k = total_dem / 2;
            match strip_solver.solve_strip(si, &dem, None, None, k) {
                Ok(paths) => {
                    strip_paths.insert(si, paths);
                }
                Err(unsat_core) => {
                    coord.add_conflict_clause(si, &dem, &unsat_core);
                    all_ok = false;
                    break;
                }
            }
        }

        if all_ok {
            final_hh_edges = hh_edges;
            final_strip_demands = strip_demands;
            break;
        }
    }

    assert!(
        !strip_paths.is_empty(),
        "Must have solved all strips successfully"
    );

    let (is_single, tours) = splice_macro_tour(
        &g,
        &decomp,
        &final_hh_edges,
        &strip_paths,
        &final_strip_demands,
        true,
    );

    // If 2-factor is formed, cycles should be valid
    assert!(!tours.is_empty(), "Splicer must produce at least one cycle");
    let total_v_in_tours: usize = tours.iter().map(|c| c.len()).sum();
    assert_eq!(
        total_v_in_tours,
        g.adjacency_list.len(),
        "All vertices must be covered in 2-factor"
    );

    if is_single {
        assert_eq!(tours.len(), 1);
        assert!(verify_tour_on_raw_graph(&tours[0], &g));
    }
}

#[test]
fn test_2opt_fast_pruning_disconnected_cycles() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 2: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Zero cross edges between C1 and C2
    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let patched = patch_cycles_2opt(cycles.clone(), &g);

    assert_eq!(patched.len(), 2, "Disconnected cycles cannot be merged");
    assert_eq!(patched, cycles, "Cycles must remain unchanged");
}

#[test]
fn test_2opt_fast_pruning_single_cross_edge() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 2: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Only 1 cross edge: (1, 4)
    g.add_edge(1, 4);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let patched = patch_cycles_2opt(cycles.clone(), &g);

    assert_eq!(
        patched.len(),
        2,
        "Cycles with single cross edge cannot be 2-opt merged"
    );
    assert_eq!(patched, cycles, "Cycles must remain unchanged");
}

#[test]
fn test_2opt_merge_with_valid_cross_edges() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 2: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // 2 valid cross edges: (1, 4) and (2, 5)
    g.add_edge(1, 4);
    g.add_edge(2, 5);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let patched = patch_cycles_2opt(cycles, &g);

    assert_eq!(patched.len(), 1, "Expected 2 cycles to merge into 1");
    assert_eq!(patched[0].len(), 6, "Expected merged tour of length 6");
    assert!(
        verify_tour_on_raw_graph(&patched[0], &g),
        "Merged tour must be valid on raw graph"
    );
}

