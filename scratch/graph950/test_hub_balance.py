#!/usr/bin/env python3
"""Tests for Global Hub Reachability and Balance Filter (Phase 1.5)."""

import collections
import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hub_balance_filter import (
    check_hub_candidate_coverage,
    verify_global_balance,
    classify_hubs,
    get_undercovered_hubs,
)


def load_graph(graph_path='/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
    G = collections.defaultdict(set)
    with open(graph_path, 'r') as f:
        for line in f:
            tokens = line.split()
            if tokens and tokens[0] == 'e':
                u, v = int(tokens[1]), int(tokens[2])
                G[u].add(v)
                G[v].add(u)
    return G


class TestHubBalance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.graph_path = '/home/ubuntu/HCP/FHCPCS-col/graph950.col'
        cls.covers_path = '/home/ubuntu/HCP/scratch/graph950/covers_multi.json'
        cls.G = load_graph(cls.graph_path)
        cls.deg = {v: len(a) for v, a in cls.G.items()}
        cls.hub_set = set(v for v, d in cls.deg.items() if d >= 20)
        with open(cls.covers_path, 'r') as f:
            cls.cover_sets = json.load(f)

    def test_tier_classification(self):
        """Verify S, B, M tier classification on graph950."""
        s_hubs, b_hubs, m_hubs, hub_set = classify_hubs(self.G, hubcut=20, s_cut=500, b_cut=100)
        self.assertEqual(len(hub_set), 310)
        self.assertEqual(len(s_hubs), 10)
        self.assertEqual(len(b_hubs), 50)
        self.assertEqual(len(m_hubs), 250)
        self.assertEqual(hub_set, s_hubs | b_hubs | m_hubs)

    def test_hub_balance_coverage(self):
        """Verify check_hub_candidate_coverage on actual Phase 1 cover sets."""
        all_ok, stats = check_hub_candidate_coverage(self.cover_sets, self.G, self.hub_set)
        self.assertTrue(all_ok, f"All hubs should be covered, but undercovered: {stats.get('undercovered_hubs')}")
        self.assertIn('min_candidates', stats)
        self.assertGreaterEqual(stats['min_candidates'], 2)
        self.assertEqual(stats['num_hubs'], 310)
        self.assertEqual(stats['num_undercovered'], 0)
        self.assertIn('tier_stats', stats)
        self.assertIn('S', stats['tier_stats'])
        self.assertIn('B', stats['tier_stats'])
        self.assertIn('M', stats['tier_stats'])
        self.assertEqual(stats['tier_stats']['S']['count'], 10)
        self.assertEqual(stats['tier_stats']['B']['count'], 50)
        self.assertEqual(stats['tier_stats']['M']['count'], 250)

    def test_verify_global_balance(self):
        """Verify verify_global_balance interface returns (True, stats)."""
        ok, stats = verify_global_balance(self.cover_sets, self.G, self.hub_set)
        self.assertTrue(ok)
        self.assertIsInstance(stats, dict)
        self.assertGreaterEqual(stats['min_candidates'], 2)

    def test_undercovered_detection_mock(self):
        """Verify detection when covers are artificially emptied or direct edges ignored."""
        # Empty cover set and no direct edges -> all hubs must be reported undercovered
        empty_covers = [[] for _ in self.cover_sets]
        all_ok, stats = check_hub_candidate_coverage(
            empty_covers, self.G, self.hub_set, include_direct=False, min_required=2
        )
        self.assertFalse(all_ok)
        self.assertEqual(stats['num_undercovered'], 310)
        self.assertEqual(stats['min_candidates'], 0)

        # Partial coverage test
        undercovered = get_undercovered_hubs(empty_covers, self.G, self.hub_set, include_direct=False, min_required=2)
        self.assertEqual(len(undercovered), 310)


if __name__ == '__main__':
    unittest.main()
