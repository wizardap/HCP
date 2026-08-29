use std::collections::HashMap;
use rustsat::types::Lit;
use cegar_fix::empirical_backbone_cutter::{EmpiricalBackboneTracker, EmpiricalBackboneCutter};

#[test]
fn test_frequency_tracking_sliding_window() {
    let mut tracker = EmpiricalBackboneTracker::new(3);
    assert_eq!(tracker.history_window, 3);
    assert_eq!(tracker.total_rounds_recorded, 0);

    // Round 1
    tracker.record_solution_edges(&[vec![1, 2, 3]]);
    assert_eq!(tracker.total_rounds_recorded, 1);
    assert_eq!(tracker.edge_history.len(), 1);

    let freq_1 = tracker.get_frequent_backbone_edges(1.0);
    assert_eq!(freq_1.len(), 3);
    assert!(freq_1.contains(&(1, 2)));
    assert!(freq_1.contains(&(2, 3)));
    assert!(freq_1.contains(&(1, 3)));

    // Round 2
    tracker.record_solution_edges(&[vec![1, 2, 4]]);
    assert_eq!(tracker.total_rounds_recorded, 2);
    assert_eq!(tracker.edge_history.len(), 2);

    let freq_round2_all = tracker.get_frequent_backbone_edges(0.5);
    assert!(freq_round2_all.contains(&(1, 2)));
    assert!(freq_round2_all.contains(&(2, 3)));
    assert!(freq_round2_all.contains(&(1, 3)));
    assert!(freq_round2_all.contains(&(2, 4)));
    assert!(freq_round2_all.contains(&(1, 4)));

    let freq_round2_high = tracker.get_frequent_backbone_edges(0.8);
    assert_eq!(freq_round2_high.len(), 1);
    assert!(freq_round2_high.contains(&(1, 2)));

    // Round 3
    tracker.record_solution_edges(&[vec![1, 2, 5]]);
    assert_eq!(tracker.total_rounds_recorded, 3);
    assert_eq!(tracker.edge_history.len(), 3);

    let freq_round3_high = tracker.get_frequent_backbone_edges(1.0);
    assert_eq!(freq_round3_high.len(), 1);
    assert!(freq_round3_high.contains(&(1, 2)));

    // Round 4 (evicts round 1 because window = 3)
    tracker.record_solution_edges(&[vec![3, 4, 5]]);
    assert_eq!(tracker.total_rounds_recorded, 4);
    assert_eq!(tracker.edge_history.len(), 3);

    // Edge (1, 2) is present in Round 2, Round 3 (count = 2 / 3 = 0.6667)
    let freq_round4_60 = tracker.get_frequent_backbone_edges(0.6);
    assert!(freq_round4_60.contains(&(1, 2)));

    let freq_round4_70 = tracker.get_frequent_backbone_edges(0.7);
    assert!(!freq_round4_70.contains(&(1, 2)));

    // Edges from round 1 only (e.g. (1, 3), (2, 3)) are completely evicted (count = 0)
    let freq_round4_any = tracker.get_frequent_backbone_edges(0.1);
    assert!(!freq_round4_any.contains(&(1, 3)));
    assert!(!freq_round4_any.contains(&(2, 3)));
}

#[test]
fn test_threshold_filtering() {
    let mut tracker = EmpiricalBackboneTracker::new(5);

    // Initial empty tracker
    assert!(tracker.get_frequent_backbone_edges(0.5).is_empty());

    // Record 5 rounds
    // Edge (10, 20) in 5/5
    // Edge (20, 30) in 4/5
    // Edge (30, 40) in 3/5
    // Edge (40, 50) in 2/5
    // Edge (50, 60) in 1/5
    tracker.record_solution_edges(&[vec![10, 20, 30, 40, 50, 60]]); // contains all 5 edges plus (10,60)
    tracker.record_solution_edges(&[vec![10, 20, 30, 40, 50, 70]]); // contains (10,20), (20,30), (30,40), (40,50)
    tracker.record_solution_edges(&[vec![10, 20, 30, 40, 70, 80]]); // contains (10,20), (20,30), (30,40)
    tracker.record_solution_edges(&[vec![10, 20, 30, 70, 80, 90]]); // contains (10,20), (20,30)
    tracker.record_solution_edges(&[vec![10, 20, 70, 80, 90, 100]]); // contains (10,20)

    assert_eq!(tracker.total_rounds_recorded, 5);
    assert_eq!(tracker.edge_history.len(), 5);

    let e_10_20 = (10, 20);
    let e_20_30 = (20, 30);
    let e_30_40 = (30, 40);
    let e_40_50 = (40, 50);
    let e_50_60 = (50, 60);

    // Threshold 1.0 (>= 5/5 = 1.0)
    let t_100 = tracker.get_frequent_backbone_edges(1.0);
    assert!(t_100.contains(&e_10_20));
    assert!(!t_100.contains(&e_20_30));

    // Threshold 0.8 (>= 4/5 = 0.8)
    let t_80 = tracker.get_frequent_backbone_edges(0.8);
    assert!(t_80.contains(&e_10_20));
    assert!(t_80.contains(&e_20_30));
    assert!(!t_80.contains(&e_30_40));

    // Threshold 0.6 (>= 3/5 = 0.6)
    let t_60 = tracker.get_frequent_backbone_edges(0.6);
    assert!(t_60.contains(&e_10_20));
    assert!(t_60.contains(&e_20_30));
    assert!(t_60.contains(&e_30_40));
    assert!(!t_60.contains(&e_40_50));

    // Threshold 0.4 (>= 2/5 = 0.4)
    let t_40 = tracker.get_frequent_backbone_edges(0.4);
    assert!(t_40.contains(&e_10_20));
    assert!(t_40.contains(&e_20_30));
    assert!(t_40.contains(&e_30_40));
    assert!(t_40.contains(&e_40_50));
    assert!(!t_40.contains(&e_50_60));

    // Threshold 0.2 (>= 1/5 = 0.2)
    let t_20 = tracker.get_frequent_backbone_edges(0.2);
    assert!(t_20.contains(&e_10_20));
    assert!(t_20.contains(&e_20_30));
    assert!(t_20.contains(&e_30_40));
    assert!(t_20.contains(&e_40_50));
    assert!(t_20.contains(&e_50_60));

    // Threshold > 1.0
    let t_above = tracker.get_frequent_backbone_edges(1.1);
    assert!(t_above.is_empty());
}

#[test]
fn test_generate_comprehensive_sec_clauses() {
    let mut lit_map: HashMap<(i32, i32), Lit> = HashMap::new();

    // Map directed edges for cycle 1: 1 -> 2 -> 3 -> 1
    // and reverse: 1 -> 3 -> 2 -> 1
    let mut lit_counter = 1;
    let mut add_bidirected = |u: i32, v: i32, map: &mut HashMap<(i32, i32), Lit>| {
        map.insert((u, v), Lit::new(lit_counter, false));
        lit_counter += 1;
        map.insert((v, u), Lit::new(lit_counter, false));
        lit_counter += 1;
    };

    // Cycle 1: [1, 2, 3]
    add_bidirected(1, 2, &mut lit_map);
    add_bidirected(2, 3, &mut lit_map);
    add_bidirected(3, 1, &mut lit_map);

    // Cycle 2: [4, 5, 6, 7]
    add_bidirected(4, 5, &mut lit_map);
    add_bidirected(5, 6, &mut lit_map);
    add_bidirected(6, 7, &mut lit_map);
    add_bidirected(7, 4, &mut lit_map);

    // Giant cycle: [10, 11, 12, 13, 14, 15] (len 6)
    add_bidirected(10, 11, &mut lit_map);
    add_bidirected(11, 12, &mut lit_map);
    add_bidirected(12, 13, &mut lit_map);
    add_bidirected(13, 14, &mut lit_map);
    add_bidirected(14, 15, &mut lit_map);
    add_bidirected(15, 10, &mut lit_map);

    let cycles = vec![
        vec![1, 2, 3],                         // len 3
        vec![4, 5, 6, 7],                      // len 4
        vec![10, 11, 12, 13, 14, 15],          // len 6 (giant)
        vec![20, 21],                          // len 2 (< 3, ignored)
    ];

    // giant_threshold = 5:
    // Subcycles of length 3 and 4 should be cut (< 5 and >= 3).
    // Giant cycle of length 6 (>= 5) should be excluded.
    let clauses = EmpiricalBackboneCutter::generate_comprehensive_sec_clauses(&cycles, 5, &lit_map);

    // 2 subcycles * 2 directions (forward + reverse) = 4 clauses
    assert_eq!(clauses.len(), 4);

    // Check forward clause for [1, 2, 3]
    let l_12 = lit_map[&(1, 2)];
    let l_23 = lit_map[&(2, 3)];
    let l_31 = lit_map[&(3, 1)];
    let expected_fwd_1 = vec![!l_12, !l_23, !l_31];

    // Check reverse clause for [1, 2, 3]
    let l_21 = lit_map[&(2, 1)];
    let l_32 = lit_map[&(3, 2)];
    let l_13 = lit_map[&(1, 3)];
    let expected_rev_1 = vec![!l_21, !l_32, !l_13];

    assert!(clauses.contains(&expected_fwd_1));
    assert!(clauses.contains(&expected_rev_1));

    // Check forward clause for [4, 5, 6, 7]
    let l_45 = lit_map[&(4, 5)];
    let l_56 = lit_map[&(5, 6)];
    let l_67 = lit_map[&(6, 7)];
    let l_74 = lit_map[&(7, 4)];
    let expected_fwd_2 = vec![!l_45, !l_56, !l_67, !l_74];

    // Check reverse clause for [4, 5, 6, 7]
    let l_54 = lit_map[&(5, 4)];
    let l_65 = lit_map[&(6, 5)];
    let l_76 = lit_map[&(7, 6)];
    let l_47 = lit_map[&(4, 7)];
    let expected_rev_2 = vec![!l_54, !l_65, !l_76, !l_47];

    assert!(clauses.contains(&expected_fwd_2));
    assert!(clauses.contains(&expected_rev_2));
}

#[test]
fn test_generate_sec_clauses_missing_literals() {
    let mut lit_map: HashMap<(i32, i32), Lit> = HashMap::new();
    // Only add forward edges for [1, 2, 3]
    lit_map.insert((1, 2), Lit::new(1, false));
    lit_map.insert((2, 3), Lit::new(2, false));
    lit_map.insert((3, 1), Lit::new(3, false));

    let cycles = vec![vec![1, 2, 3]];
    let clauses = EmpiricalBackboneCutter::generate_comprehensive_sec_clauses(&cycles, 10, &lit_map);

    // Only forward clause generated because reverse edges are missing in lit_map
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0], vec![!Lit::new(1, false), !Lit::new(2, false), !Lit::new(3, false)]);
}
