use cegar_fix::cut_selector::{CutSelector, CutSelectorOptions};
use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;

#[test]
fn test_cut_selector_budget_capping() {
    let mut g = Graph::new();
    let mut cycles = Vec::new();

    // Create 100 disjoint triangles (nodes 3*i+1, 3*i+2, 3*i+3)
    for i in 0..100 {
        let v1 = 3 * i + 1;
        let v2 = 3 * i + 2;
        let v3 = 3 * i + 3;
        g.add_edge(v1, v2);
        g.add_edge(v2, v3);
        g.add_edge(v3, v1);
        cycles.push(vec![v1, v2, v3]);
    }

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // Default max_cuts_per_round is 40
    let options = CutSelectorOptions::default();
    assert_eq!(options.max_cuts_per_round, 40);

    let (clauses, selected_cycles) =
        CutSelector::select_and_generate_cuts(&cycles, &g, &encoder, &options);

    assert_eq!(selected_cycles.len(), 40);
    assert!(!clauses.is_empty());

    // Custom budget: 15
    let custom_options = CutSelectorOptions {
        max_cuts_per_round: 15,
        ..CutSelectorOptions::default()
    };
    let (_, selected_15) =
        CutSelector::select_and_generate_cuts(&cycles, &g, &encoder, &custom_options);
    assert_eq!(selected_15.len(), 15);
}

#[test]
fn test_cut_selector_short_cycle_priority() {
    let mut g = Graph::new();

    // Cycle of length 2 (should be skipped, len < 3)
    let c_len2 = vec![1, 2];
    g.add_edge(1, 2);

    // Cycle of length 3
    let c_len3 = vec![10, 11, 12];
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 10);

    // Cycle of length 4
    let c_len4 = vec![20, 21, 22, 23];
    g.add_edge(20, 21);
    g.add_edge(21, 22);
    g.add_edge(22, 23);
    g.add_edge(23, 20);

    // Cycle of length 5
    let c_len5 = vec![30, 31, 32, 33, 34];
    g.add_edge(30, 31);
    g.add_edge(31, 32);
    g.add_edge(32, 33);
    g.add_edge(33, 34);
    g.add_edge(34, 30);

    // Cycle of length 100 (should be skipped, default max_cycle_len_for_cut = 64)
    let mut c_len100 = Vec::new();
    for i in 100..200 {
        c_len100.push(i);
        let next = if i == 199 { 100 } else { i + 1 };
        g.add_edge(i, next);
    }

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // Pass in mixed / unsorted order
    let input_cycles = vec![
        c_len100.clone(),
        c_len4.clone(),
        c_len2.clone(),
        c_len5.clone(),
        c_len3.clone(),
    ];

    let (clauses, selected_cycles) = CutSelector::select_and_generate_cuts(
        &input_cycles,
        &g,
        &encoder,
        &CutSelectorOptions::default(),
    );

    // Should contain c_len3, c_len4, c_len5 in ascending order of length
    assert_eq!(selected_cycles.len(), 3);
    assert_eq!(selected_cycles[0], c_len3);
    assert_eq!(selected_cycles[1], c_len4);
    assert_eq!(selected_cycles[2], c_len5);
    assert!(!clauses.is_empty());
}

#[test]
fn test_cut_selector_boundary_cuts() {
    let mut g = Graph::new();
    // 4-node graph: Triangle 1-2-3-1 connected to node 4 via (3,4) and (4,1)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let triangle = vec![1, 2, 3];
    let cycles = vec![triangle.clone()];

    // 1. Boundary cuts enabled (default):
    // Direct clause: ¬x_{1->2} ∨ ¬x_{2->3} ∨ ¬x_{3->1}
    // Outgoing cut edges from {1, 2, 3} to {4} are (1, 4) and (3, 4) -> boundary disjunction: x_{1->4} ∨ x_{3->4}
    let options = CutSelectorOptions::default();
    let (clauses, selected) =
        CutSelector::select_and_generate_cuts(&cycles, &g, &encoder, &options);

    assert_eq!(selected, vec![triangle.clone()]);
    assert_eq!(clauses.len(), 2);

    let lit_1_2 = encoder.graph_lit_map[&(1, 2)];
    let lit_2_3 = encoder.graph_lit_map[&(2, 3)];
    let lit_3_1 = encoder.graph_lit_map[&(3, 1)];
    let lit_1_4 = encoder.graph_lit_map[&(1, 4)];
    let lit_3_4 = encoder.graph_lit_map[&(3, 4)];

    let mut expected_direct = vec![!lit_1_2, !lit_2_3, !lit_3_1];
    expected_direct.sort_unstable();

    // Verify direct blocking clause is present
    let direct_found = clauses.iter().any(|c| {
        let mut lits: Vec<_> = c.iter().copied().collect();
        lits.sort_unstable();
        lits == expected_direct
    });
    assert!(direct_found, "Direct blocking clause must be generated");

    // Verify boundary disjunction clause is present
    let mut expected_boundary = vec![lit_1_4, lit_3_4];
    expected_boundary.sort_unstable();

    let boundary_found = clauses.iter().any(|c| {
        let mut lits: Vec<_> = c.iter().copied().collect();
        lits.sort_unstable();
        lits == expected_boundary
    });
    assert!(boundary_found, "Boundary disjunction clause must be generated");

    // 2. Boundary cuts disabled: only 1 direct clause
    let no_boundary_options = CutSelectorOptions {
        enable_boundary_cuts: false,
        ..CutSelectorOptions::default()
    };
    let (no_b_clauses, _) =
        CutSelector::select_and_generate_cuts(&cycles, &g, &encoder, &no_boundary_options);
    assert_eq!(no_b_clauses.len(), 1);

    // 3. |delta(C)| > 2 case: Add node 5 with edges to node 2 and node 3
    let mut g2 = Graph::new();
    g2.add_edge(1, 2);
    g2.add_edge(2, 3);
    g2.add_edge(3, 1);
    g2.add_edge(3, 4);
    g2.add_edge(4, 1);
    g2.add_edge(2, 5);
    g2.add_edge(5, 3);

    let mut encoder2 = Encoder::new();
    let _ = encoder2.encode(&g2, 0, 0, 0, 0, 0, 0);

    // Cut edges from {1, 2, 3} are: (1, 4), (3, 4), (2, 5), (3, 5) -> size 4 > 2
    let (clauses_gt2, _) =
        CutSelector::select_and_generate_cuts(&cycles, &g2, &encoder2, &options);
    assert_eq!(clauses_gt2.len(), 2); // 1 direct clause + 1 boundary clause (disjunction)

    let lit_2_5 = encoder2.graph_lit_map[&(2, 5)];
    let lit_3_5 = encoder2.graph_lit_map[&(3, 5)];
    let lit2_1_4 = encoder2.graph_lit_map[&(1, 4)];
    let lit2_3_4 = encoder2.graph_lit_map[&(3, 4)];

    let mut expected_boundary_lits = vec![lit2_1_4, lit2_3_4, lit_2_5, lit_3_5];
    expected_boundary_lits.sort_unstable();

    let boundary_disjunction_found = clauses_gt2.iter().any(|c| {
        let mut lits: Vec<_> = c.iter().copied().collect();
        lits.sort_unstable();
        lits == expected_boundary_lits
    });
    assert!(
        boundary_disjunction_found,
        "Boundary disjunction clause for |delta(C)| > 2 must be generated"
    );
}

#[test]
fn test_cut_selector_empty_and_thresholds() {
    let g = Graph::new();
    let encoder = Encoder::new();
    let empty_cycles: Vec<Vec<i32>> = Vec::new();

    let (clauses, selected) = CutSelector::select_and_generate_cuts(
        &empty_cycles,
        &g,
        &encoder,
        &CutSelectorOptions::default(),
    );
    assert!(clauses.is_empty());
    assert!(selected.is_empty());

    // Test small_cycle_threshold: if cycle len > small_cycle_threshold, boundary cut is skipped
    let mut g_thresh = Graph::new();
    // 10-cycle with external node 99
    let mut cycle_10 = Vec::new();
    for i in 1..=10 {
        cycle_10.push(i);
        let next = if i == 10 { 1 } else { i + 1 };
        g_thresh.add_edge(i, next);
    }
    g_thresh.add_edge(1, 99);
    g_thresh.add_edge(99, 2);

    let mut enc_thresh = Encoder::new();
    let _ = enc_thresh.encode(&g_thresh, 0, 0, 0, 0, 0, 0);

    // Default small_cycle_threshold is 8, cycle len is 10 -> boundary cut skipped, only direct clause
    let opts = CutSelectorOptions {
        max_cycle_len_for_cut: 20,
        small_cycle_threshold: 8,
        enable_boundary_cuts: true,
        ..CutSelectorOptions::default()
    };
    let (clauses_thresh, selected_thresh) = CutSelector::select_and_generate_cuts(
        &[cycle_10.clone()],
        &g_thresh,
        &enc_thresh,
        &opts,
    );
    assert_eq!(selected_thresh.len(), 1);
    assert_eq!(clauses_thresh.len(), 1); // direct only
}

#[test]
fn test_cut_selector_fallback_when_all_cycles_exceed_max_len() {
    let mut g = Graph::new();
    let mut cycle_70 = Vec::new();
    for i in 1..=70 {
        cycle_70.push(i);
        let next = if i == 70 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }
    let mut enc = Encoder::new();
    let _ = enc.encode(&g, 0, 0, 0, 0, 0, 0);

    // max_cycle_len_for_cut is 64, but cycle is 70.
    // Fallback should still select the single shortest cycle and generate a direct blocking clause!
    let opts = CutSelectorOptions {
        max_cycle_len_for_cut: 64,
        ..CutSelectorOptions::default()
    };
    let (clauses, selected) = CutSelector::select_and_generate_cuts(
        &[cycle_70.clone()],
        &g,
        &enc,
        &opts,
    );
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0], cycle_70);
    assert_eq!(clauses.len(), 1);
}

