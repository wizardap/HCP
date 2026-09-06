import sys, os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from scratch.solve_graph788_dp_bitmask import (
    load_graph,
    setup_stage1,
    acquire_giant_backbone,
    decompose_subcycle_components,
    generate_component_routes,
    run_dp_bitmask_splicer,
    get_cycles_from_edges,
)

def test_dp_engine():
    G, d = load_graph('FHCPCS-col/graph788.col')
    blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, d)
    edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)
    comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
    comp_routes = generate_component_routes(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
    final_edges = run_dp_bitmask_splicer(edges, blocks, node_to_block_end, comps, comp_routes)
    final_cycs, _ = get_cycles_from_edges(final_edges, blocks, node_to_block_end)
    assert len(final_cycs) == 1, f"Expected 1 cycle, got {len(final_cycs)}"
    assert len(final_cycs[0]) // 2 == 1540, f"Expected 1540 blocks, got {len(final_cycs[0]) // 2}"
    print("Stage 3 DP Bitmask test PASSED: 1540 edges generated and exactly 1 Hamiltonian cycle verified.")

if __name__ == '__main__':
    test_dp_engine()
