use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::static_cycle_cutter::StaticCycleCutter;

#[test]
fn test_hub_aware_selective_static_cycle_cuts() {
    let mut g = Graph::new();
    
    // Hub vertex H = 100 with degree 12
    let hub = 100;
    // Hub connects to 1 and 5, plus dummy leaves 6..15 (so degree = 12 >= 10)
    g.add_edge(hub, 1);
    g.add_edge(hub, 5);
    for v in 6..=15 {
        g.add_edge(hub, v);
    }
    
    // Form a chordless 6-cycle touching Hub: 100 - 1 - 2 - 3 - 4 - 5 - 100
    // Nodes 2, 3, 4 are NOT connected to 100.
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);

    // Form an independent peripheral 4-cycle not touching Hub: 20 - 21 - 22 - 23 - 20
    g.add_edge(20, 21);
    g.add_edge(21, 22);
    g.add_edge(22, 23);
    g.add_edge(23, 20);

    // Add extra vertices to make total_v > 8
    g.add_edge(30, 31);
    g.add_edge(31, 32);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // 1. Unconstrained: should generate cuts for both 4-cycle and 6-cycle
    let cuts_unconstrained = StaticCycleCutter::generate_selective_static_cycle_cuts(&g, &encoder, usize::MAX);
    // Peripheral 4-cycle: 2 clauses. 6-cycle touching Hub: 2 clauses. Total >= 4
    assert!(cuts_unconstrained.len() >= 4, "Unconstrained should find both 4-cycle and 6-cycle");

    // 2. Hub-aware with hub_deg_threshold = 10:
    // Hub 100 has deg 12 >= 10. The 6-cycle contains Hub 100, so it MUST be skipped.
    // The peripheral 4-cycle does NOT contain Hub 100 (and 4-cycles are preserved), so its 2 clauses are kept.
    let cuts_throttled = StaticCycleCutter::generate_selective_static_cycle_cuts(&g, &encoder, 10);
    
    // Exactly 2 clauses (the peripheral 4-cycle)
    assert_eq!(cuts_throttled.len(), 2, "Expected exactly 2 clauses for peripheral 4-cycle, 6-cycle through Hub must be throttled");
}
