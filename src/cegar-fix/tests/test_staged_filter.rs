use cegar_fix::staged_subcycle_filter::{StagedSubcycleFilter, Subcycle};

#[test]
fn test_staged_subcycle_extraction_and_progression() {
    // 2-factor with one 2-cycle (1<->2) and one 4-cycle (3->4->5->6->3)
    let arcs = vec![
        (1, 2), (2, 1),
        (3, 4), (4, 5), (5, 6), (6, 3),
    ];
    let cycles = StagedSubcycleFilter::extract_subcycles(&arcs);
    assert_eq!(cycles.len(), 2);

    let mut filter = StagedSubcycleFilter::new(500);
    assert_eq!(filter.k_stage, 2);

    // Round 1: K_stage = 2 should only select the 2-cycle
    let active = filter.filter_active_cycles(&cycles, 6);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].vertices.len(), 2);

    // Suppose 2-cycle is eliminated, now only 4-cycle exists
    let cycles_round2 = vec![cycles[1].clone()];
    let active2 = filter.filter_active_cycles(&cycles_round2, 6);
    assert_eq!(filter.k_stage, 4);
    assert_eq!(active2.len(), 1);
    assert_eq!(active2[0].vertices.len(), 4);
}

#[test]
fn test_staged_filter_edge_cases() {
    // Single cycle of total length N => filter_active_cycles should return empty
    let single_cycle = vec![
        Subcycle {
            vertices: vec![1, 2, 3, 4, 5, 6],
            edges: vec![(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 1)],
        }
    ];
    let mut filter = StagedSubcycleFilter::new(10);
    let active = filter.filter_active_cycles(&single_cycle, 6);
    assert!(active.is_empty());

    // Multiple cycles of larger lengths, requiring progression jumps
    let cycles = vec![
        Subcycle {
            vertices: vec![1, 2, 3, 4, 5],
            edges: vec![(1, 2), (2, 3), (3, 4), (4, 5), (5, 1)],
        },
        Subcycle {
            vertices: vec![6, 7, 8, 9, 10],
            edges: vec![(6, 7), (7, 8), (8, 9), (9, 10), (10, 6)],
        },
    ];
    let mut filter2 = StagedSubcycleFilter::new(1);
    // K_stage starts at 2. Since cycles are length 5, stage doubles: 2 -> 4 -> 8.
    let active2 = filter2.filter_active_cycles(&cycles, 10);
    assert_eq!(filter2.k_stage, 8);
    // max_batch_size is 1, so only 1 cycle returned
    assert_eq!(active2.len(), 1);
    assert_eq!(active2[0].vertices.len(), 5);
}
