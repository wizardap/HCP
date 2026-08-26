use cegar_fix::component_meta_graph::ComponentMetaGraph;
use cegar_fix::graph::Graph;

#[test]
fn test_component_meta_graph_disconnected() {
    let mut g = Graph::new();
    // Cycle 0: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 1: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let meta_graph = ComponentMetaGraph::build(&cycles, &g);

    assert_eq!(meta_graph.num_components, 2);
    assert_eq!(meta_graph.is_connected(), false);
    assert_eq!(meta_graph.get_meta_components().len(), 2);
    assert_eq!(meta_graph.has_merge_potential(0, 1), false);
}

#[test]
fn test_component_meta_graph_connected() {
    let mut g = Graph::new();
    // Cycle 0: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 1: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // Cycle 2: 7 - 8 - 9 - 7
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 7);

    // Cross-edges: 2 between cycle 0 and 1
    g.add_edge(1, 4);
    g.add_edge(2, 5);

    // Cross-edges: 2 between cycle 1 and 2
    g.add_edge(5, 7);
    g.add_edge(6, 8);

    // Cross-edges: 2 between cycle 0 and 2
    g.add_edge(3, 9);
    g.add_edge(1, 8);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    let meta_graph = ComponentMetaGraph::build(&cycles, &g);

    assert_eq!(meta_graph.num_components, 3);
    assert_eq!(meta_graph.is_connected(), true);
    assert_eq!(meta_graph.get_meta_components().len(), 1);
    assert_eq!(meta_graph.has_merge_potential(0, 1), true);
    assert_eq!(meta_graph.has_merge_potential(1, 2), true);
    assert_eq!(meta_graph.has_merge_potential(0, 2), true);
}

#[test]
fn test_component_meta_graph_merge_potential() {
    let mut g = Graph::new();
    // Cycle 0: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Cycle 1: 4 - 5 - 6 - 4
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    // 1 cross edge between cycle 0 and 1
    g.add_edge(1, 4);

    let cycles = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let meta_graph = ComponentMetaGraph::build(&cycles, &g);

    assert_eq!(meta_graph.is_connected(), false);
    assert_eq!(meta_graph.get_meta_components().len(), 2);
    assert_eq!(meta_graph.has_merge_potential(0, 1), false);

    // Add a second cross edge
    g.add_edge(2, 5);
    let meta_graph2 = ComponentMetaGraph::build(&cycles, &g);
    assert_eq!(meta_graph2.is_connected(), true);
    assert_eq!(meta_graph2.get_meta_components().len(), 1);
    assert_eq!(meta_graph2.has_merge_potential(0, 1), true);
}
