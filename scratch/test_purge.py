import os
import unittest

class TestPurge(unittest.TestCase):
    def test_no_precomputed_data_corridors(self):
        self.assertFalse(os.path.exists("data/corridors"), "data/corridors must be purged")

    def test_no_legacy_corridor_solver(self):
        self.assertFalse(os.path.exists("hcp_solver/families/corridor_solver.py"), "corridor_solver.py must be purged")

    def test_pipeline_has_no_family_imports(self):
        with open("hcp_solver/core/pipeline.py", "r") as f:
            content = f.read()
        self.assertNotIn("families.dense_bipartite", content)
        self.assertNotIn("families.corridor_solver", content)
        self.assertNotIn("families.block_splicer", content)

if __name__ == "__main__":
    unittest.main()
