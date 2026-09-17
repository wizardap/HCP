import unittest

class TestFreezeInvariants(unittest.TestCase):
    def test_invariant_detection(self):
        with open("src/cegar-fix/examples/solve_class1_exact.rs") as f:
            content = f.read()
        self.assertIn("frozen_giant_edges", content)
        self.assertIn("Frozen invariant Giant edges", content)

if __name__ == '__main__':
    unittest.main()
