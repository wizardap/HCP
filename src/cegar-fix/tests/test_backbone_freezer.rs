use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::backbone_freezer::BackboneFreezer;

#[test]
fn test_backbone_freezer_extraction() {
    let mut g = Graph::new();
    // Giant cycle: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 1
    for i in 1..=7 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(8, 1);

    // Small cycle: 9 -> 10 -> 9
    g.add_edge(9, 10);

    // Connection from small cycle to giant cycle only at node 1
    g.add_edge(1, 9);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let giant = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let small = vec![9, 10];
    let cycles = vec![giant, small];

    // Nodes 1, 2, 8 are near the boundary (connected to 9).
    // Nodes 4 -> 5 -> 6 should be deep in the internal backbone.
    let assumps = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.5);

    assert!(!assumps.is_empty(), "Internal backbone edges should be extracted");
}
