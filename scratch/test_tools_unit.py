#!/usr/bin/env python3
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from benchmark_runner_ablation import (
    parse_col_graph,
    parse_hcp_tour,
    verify_tour,
    select_graphs,
    parse_duration_to_seconds,
)
from analyze_ablation_results import (
    compute_par2,
    manual_wilcoxon_signed_rank,
    perform_wilcoxon_test,
    export_cactus_csv,
    analyze_records,
)

class TestBenchmarkTools(unittest.TestCase):
    def test_parse_and_verify_tour_valid(self):
        # Triangle graph: 1-2, 2-3, 3-1
        adj = {1: {2, 3}, 2: {1, 3}, 3: {1, 2}}
        valid, msg = verify_tour([1, 2, 3], 3, adj)
        self.assertTrue(valid)

    def test_verify_tour_invalid_duplicate(self):
        adj = {1: {2, 3}, 2: {1, 3}, 3: {1, 2}}
        valid, msg = verify_tour([1, 2, 2], 3, adj)
        self.assertFalse(valid)
        self.assertIn("Duplicate", msg)

    def test_verify_tour_invalid_missing_edge(self):
        adj = {1: {2}, 2: {1, 3}, 3: {2}}  # No edge (3, 1)
        valid, msg = verify_tour([1, 2, 3], 3, adj)
        self.assertFalse(valid)
        self.assertIn("does not exist", msg)

    def test_verify_tour_invalid_length(self):
        adj = {1: {2, 3}, 2: {1, 3}, 3: {1, 2}}
        valid, msg = verify_tour([1, 2], 3, adj)
        self.assertFalse(valid)
        self.assertIn("length", msg)

    def test_parse_duration(self):
        self.assertAlmostEqual(parse_duration_to_seconds("1.5", "s"), 1.5)
        self.assertAlmostEqual(parse_duration_to_seconds("250", "ms"), 0.25)
        self.assertAlmostEqual(parse_duration_to_seconds("500", "µs"), 0.0005)
        self.assertAlmostEqual(parse_duration_to_seconds("1000", "ns"), 0.000001)

    def test_select_graphs_sample(self):
        repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        graphs_dir = os.path.join(repo_root, "FHCPCS-col")
        sampled = select_graphs(None, 3, graphs_dir)
        self.assertEqual(len(sampled), 3)
        self.assertTrue(sampled[0].endswith("graph1.col"))
        self.assertTrue(sampled[1].endswith("graph501.col"))
        self.assertTrue(sampled[2].endswith("graph1001.col"))

    def test_compute_par2(self):
        # 1 solved in 10s, 1 timeout with limit 30s -> PAR-2 = (10 + 2*30) / 2 = 35s
        records = [
            {"status": "SATISFIABLE", "verified": True, "wall_time": 10.0, "timeout_limit": 30.0},
            {"status": "TIMEOUT", "verified": None, "wall_time": 30.0, "timeout_limit": 30.0},
        ]
        par2, costs = compute_par2(records, 30.0)
        self.assertAlmostEqual(par2, 35.0)

    def test_wilcoxon_paired(self):
        records_c3 = [
            {"graph": "g1", "seed": 1, "status": "SATISFIABLE", "verified": True, "wall_time": 5.0, "timeout_limit": 30.0},
            {"graph": "g2", "seed": 1, "status": "SATISFIABLE", "verified": True, "wall_time": 12.0, "timeout_limit": 30.0},
            {"graph": "g3", "seed": 1, "status": "SATISFIABLE", "verified": True, "wall_time": 8.0, "timeout_limit": 30.0},
        ]
        records_c0 = [
            {"graph": "g1", "seed": 1, "status": "TIMEOUT", "verified": None, "wall_time": 30.0, "timeout_limit": 30.0},
            {"graph": "g2", "seed": 1, "status": "TIMEOUT", "verified": None, "wall_time": 30.0, "timeout_limit": 30.0},
            {"graph": "g3", "seed": 1, "status": "TIMEOUT", "verified": None, "wall_time": 30.0, "timeout_limit": 30.0},
        ]
        res = perform_wilcoxon_test(records_c3, records_c0, "C3", "C0", 30.0)
        self.assertEqual(res["pairs_count"], 3)
        self.assertEqual(res["wins_a"], 3)
        self.assertEqual(res["wins_b"], 0)

    def test_cactus_export(self):
        records = [
            {"condition": 0, "status": "SATISFIABLE", "verified": True, "wall_time": 2.5},
            {"condition": 3, "status": "SATISFIABLE", "verified": True, "wall_time": 1.1},
            {"condition": 3, "status": "SATISFIABLE", "verified": True, "wall_time": 0.5},
        ]
        with tempfile.NamedTemporaryFile(suffix=".csv", mode="w+", delete=False) as tf:
            csv_path = tf.name
        try:
            export_cactus_csv(records, csv_path, [0, 1, 2, 3])
            with open(csv_path) as fp:
                lines = [line.strip() for line in fp.readlines()]
            self.assertEqual(lines[0], "solved_instance_idx,c0_time,c1_time,c2_time,c3_time")
            self.assertTrue(lines[1].startswith("1,2.500000,,,"))
        finally:
            if os.path.exists(csv_path):
                os.remove(csv_path)

if __name__ == '__main__':
    unittest.main()
