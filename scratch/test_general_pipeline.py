"""
Unit test for General, Router-Free HCP Solver Pipeline.
"""

import os
import sys
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO_ROOT)

from hcp_solver import solve_general_hcp, Graph, verify_tour

class TestGeneralPipeline(unittest.TestCase):
    def test_small_in_memory_cycle(self):
        # 6-vertex cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
        adj = {
            1: {2, 6},
            2: {1, 3},
            3: {2, 4},
            4: {3, 5},
            5: {4, 6},
            6: {5, 1},
        }
        G = Graph(adj, "test_cycle_6")
        tour = solve_general_hcp(adj, timeout_sec=10.0, verbose=False)
        self.assertIsNotNone(tour)
        self.assertEqual(len(tour), 6)
        ok, msg = verify_tour(tour, G)
        self.assertTrue(ok, f"Verification failed: {msg}")

    def test_in_memory_graph_with_chords(self):
        # 8-vertex Hamiltonian graph with chords
        adj = {
            1: {2, 8, 5},
            2: {1, 3, 6},
            3: {2, 4, 7},
            4: {3, 5, 8},
            5: {4, 6, 1},
            6: {5, 7, 2},
            7: {6, 8, 3},
            8: {7, 1, 4},
        }
        G = Graph(adj, "test_chords_8")
        tour = solve_general_hcp(adj, timeout_sec=10.0, verbose=False)
        self.assertIsNotNone(tour)
        self.assertEqual(len(tour), 8)
        ok, msg = verify_tour(tour, G)
        self.assertTrue(ok, f"Verification failed: {msg}")

    def test_general_benchmark_graph1(self):
        col_path = os.path.join(REPO_ROOT, "FHCPCS-col", "graph1.col")
        if os.path.exists(col_path):
            tour = solve_general_hcp(col_path, timeout_sec=15.0, verbose=False)
            self.assertIsNotNone(tour)
            self.assertEqual(len(tour), 66)

    def test_challenge_benchmark_graph710(self):
        col_path = os.path.join(REPO_ROOT, "FHCPCS-col", "graph710.col")
        if os.path.exists(col_path):
            tour = solve_general_hcp(col_path, timeout_sec=15.0, verbose=False)
            self.assertIsNotNone(tour)
            self.assertEqual(len(tour), 4064)

if __name__ == "__main__":
    unittest.main()
