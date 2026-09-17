import unittest, pickle
from solve_class1_dp_bitmask import (
    load_graph, setup_stage1_contraction, acquire_giant_backbone,
    decompose_subcycle_components, generate_component_routes_local_sat,
    get_cycles_from_edges
)

class TestStage4LocalSAT(unittest.TestCase):
    def test_local_sat_route_generation(self):
        col_file = 'FHCPCS-col/graph868.col'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        
        routes_map = generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
        self.assertEqual(len(routes_map), len(comps), "Every component must have routes")
        for c_id in range(len(comps)):
            routes = routes_map[c_id]
            self.assertGreaterEqual(len(routes), 1, f"Component {c_id} must have at least 1 valid route")
            # Verify validity of first route when applied individually
            r = routes[0]
            test_edges = (edges - r['removed']) | r['added']
            self.assertEqual(len(test_edges), len(edges))
            new_cycs, _ = get_cycles_from_edges(test_edges, blocks, node_to_block_end)
            # Applying one route must reduce cycle count
            self.assertLess(len(new_cycs), len(cycs), f"Route for comp {c_id} must decrease cycle count")

if __name__ == '__main__':
    unittest.main()
