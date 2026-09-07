# scratch/test_stage3_components.py
import unittest, pickle
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components

class TestStage3Components(unittest.TestCase):
    def test_component_decomposition(self):
        col_file = 'FHCPCS-col/graph868.col'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        self.assertEqual(len(comps), 20, f"Expected 20 independent components, got {len(comps)}")
        
        total_subs = sum(len(c) for c in comps)
        self.assertEqual(total_subs, 30, f"Expected 30 total subcycles across components, got {total_subs}")

if __name__ == '__main__':
    unittest.main()
