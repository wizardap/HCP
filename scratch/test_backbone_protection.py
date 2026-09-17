import unittest

class TestBackboneProtection(unittest.TestCase):
    def test_giant_not_cut(self):
        with open("src/cegar-fix/examples/solve_class1_exact.rs") as f:
            content = f.read()
        self.assertIn("let cycles_to_cut = &cycles[1..];", content)
        self.assertNotIn("let has_giant = cycles[0].len() > n_dir / 2;", content)
        self.assertIn("c.len() <= n_dir / 2", content)

if __name__ == '__main__':
    unittest.main()
