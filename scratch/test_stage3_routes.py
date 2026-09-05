import sys, os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from scratch.solve_graph788_dp_bitmask import (
    load_graph,
    setup_stage1,
    acquire_giant_backbone,
    decompose_subcycle_components,
    generate_component_routes,
)

def test_decomposition():
    G, d = load_graph('FHCPCS-col/graph788.col')
    blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, d)
    edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)
    comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
    assert len(comps) == 9, f"Expected 9 components, got {len(comps)}"
    print("Stage 3 decomposition test PASSED: Exactly 9 independent components verified.")

def test_generate_routes():
    G, d = load_graph('FHCPCS-col/graph788.col')
    blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, d)
    edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)
    comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
    comp_routes = generate_component_routes(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
    assert len(comp_routes) == 9, f"Expected 9 component route entries, got {len(comp_routes)}"
    total_routes = sum(len(r) for r in comp_routes.values())
    print(f"Stage 3 route table test PASSED: {total_routes} total candidate routes across 9 components.")

if __name__ == '__main__':
    test_decomposition()
    test_generate_routes()
