use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::bridge_cut_generator::BridgeCutGenerator;

#[test]
fn test_bridge_cut_generator() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Small subcycle: 5 - 6 - 5
    g.add_edge(5, 6);

    // Bridge edges between giant cycle and small subcycle: (2, 5) and (3, 6)
    g.add_edge(2, 5);
    g.add_edge(3, 6);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let giant = vec![1, 2, 3, 4];
    let small = vec![5, 6];
    let cycles = vec![giant, small];

    let cuts = BridgeCutGenerator::generate_bridge_cuts(&cycles, &g, &encoder);
    assert_eq!(cuts.len(), 2, "Should generate 1 entry clause and 1 exit clause");
}
