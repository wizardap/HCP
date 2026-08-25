use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::snark_bridge::SnarkBridgeEngine;

#[test]
fn test_snark_bridge_detection_and_locking() {
    let mut g = Graph::new();
    // 3-regular base cubic cycle on 6 vertices: 1-2-3-4-5-6-1 with chords 1-4, 2-5, 3-6
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 1);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);
    
    // Add 1 extra edge between 1 and 3 making deg(1)=4 and deg(3)=4, while all others remain 3
    g.add_edge(1, 3);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let bridge = SnarkBridgeEngine::detect_and_extract_key_bridge(&g, &encoder);
    assert!(bridge.is_some(), "Should detect key bridge between degree-4 vertices");
    let (u, v, _lit) = bridge.unwrap();
    assert!((u == 1 && v == 3) || (u == 3 && v == 1));
}

#[test]
fn test_snark_bridge_regular_graph_none() {
    let mut g = Graph::new();
    // Pure 3-regular graph
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 4);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let bridge = SnarkBridgeEngine::detect_and_extract_key_bridge(&g, &encoder);
    assert!(bridge.is_none(), "Pure regular graph should have no key bridge");
}

#[test]
fn test_snark_bridge_non_matching_topology() {
    // Test with a degree 2 vertex present
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    assert!(SnarkBridgeEngine::detect_and_extract_key_bridge(&g, &encoder).is_none());
}
