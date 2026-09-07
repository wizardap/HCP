import unittest, os, pickle, collections, sys
sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone

class TestStage1And2(unittest.TestCase):
    def test_contraction_and_backbone(self):
        col_file = 'FHCPCS-col/graph868.col'
        self.assertTrue(os.path.exists(col_file))
        G, degs = load_graph(col_file)
        self.assertEqual(len(G), 5544)
        
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        self.assertEqual(len(blocks), 1848)
        self.assertEqual(len(node_to_block_end), 3696)
        
        pkl_path = 'scratch/graph868_giant_1686.pkl'
        self.assertTrue(os.path.exists(pkl_path))
        with open(pkl_path, 'rb') as f:
            data = pickle.load(f)
        edges = set(tuple(sorted(e)) for e in data['edges'])
        self.assertEqual(len(edges), 1848)
        
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        giant_len = len(cycs[giant_idx]) // 2
        self.assertEqual(giant_len, 1686)
        self.assertEqual(len(cycs), 31)

if __name__ == '__main__':
    unittest.main()
