use cegar_fix::graph::Graph;
use cegar_fix::auto_classifier::{AutoTopologyClassifier, TargetTrack};

#[test]
fn test_synthetic_classification() {
    let mut g_b1 = Graph::new();
    // Create ladder structure with 60 high-degree hubs
    for h in 1..=60 {
        for v in 100..120 {
            g_b1.add_edge(h, v);
        }
    }
    let feat_b1 = AutoTopologyClassifier::extract_features(&g_b1);
    assert_eq!(AutoTopologyClassifier::classify(&feat_b1), TargetTrack::B1LadderTwoTier);

    let mut g_gadget = Graph::new();
    for i in 1..=10 {
        let next = if i == 10 { 1 } else { i + 1 };
        g_gadget.add_edge(i, next);
    }
    g_gadget.add_edge(1, 5); // creates degree-2 vertices
    let feat_gadget = AutoTopologyClassifier::extract_features(&g_gadget);
    assert_eq!(AutoTopologyClassifier::classify(&feat_gadget), TargetTrack::GadgetInterfaceParity);

    let mut g_snark = Graph::new();
    g_snark.add_edge(1, 2); g_snark.add_edge(2, 3); g_snark.add_edge(3, 4);
    g_snark.add_edge(4, 5); g_snark.add_edge(5, 6); g_snark.add_edge(6, 1);
    g_snark.add_edge(1, 4); g_snark.add_edge(2, 5); g_snark.add_edge(3, 6);
    g_snark.add_edge(1, 3);
    let feat_snark = AutoTopologyClassifier::extract_features(&g_snark);
    assert_eq!(AutoTopologyClassifier::classify(&feat_snark), TargetTrack::SnarkKeyBridge);

    let mut g_sparse = Graph::new();
    // Create large 3-regular cycle graph (density = 1.5, n = 2000)
    for i in 1..=2000 {
        let next = if i == 2000 { 1 } else { i + 1 };
        g_sparse.add_edge(i, next);
        let cross = (i + 500) % 2000 + 1;
        g_sparse.add_edge(i, cross);
    }
    let feat_sparse = AutoTopologyClassifier::extract_features(&g_sparse);
    assert_eq!(AutoTopologyClassifier::classify(&feat_sparse), TargetTrack::GeneralCaDiCaL);
}

#[test]
fn test_general_fallback_classification() {
    let mut g_general = Graph::new();
    // Small 3-regular graph with 100 vertices (n < 1000, hubs = 0)
    for i in 1..=100 {
        let next = if i == 100 { 1 } else { i + 1 };
        g_general.add_edge(i, next);
        let cross = (i + 25) % 100 + 1;
        g_general.add_edge(i, cross);
    }
    let feat_general = AutoTopologyClassifier::extract_features(&g_general);
    assert_eq!(AutoTopologyClassifier::classify(&feat_general), TargetTrack::GeneralCaDiCaL);
}

#[test]
fn test_feature_extraction_details() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);
    g.add_edge(1, 3); // 1 and 3 degree 3; 2 and 4 degree 2

    let feat = AutoTopologyClassifier::extract_features(&g);
    assert_eq!(feat.n, 4);
    assert_eq!(feat.m, 5);
    assert_eq!(feat.max_degree, 3);
    assert_eq!(feat.degree2_count, 2);
    assert_eq!(feat.deg3_count, 2);
    assert_eq!(feat.deg4_count, 0);
    assert_eq!(feat.hub_count, 0);
    assert!((feat.density - 1.25).abs() < 1e-6);
}

