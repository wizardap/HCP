import collections
import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph, DecompositionResult
from scratch.graph950.macro_splicer import verify_tour_on_raw_graph, splice_macro_tour, splice_and_verify_tour

def test_verifier_detects_valid_and_invalid_tours():
    G, _ = load_graph('FHCPCS-col/graph950.col')
    n = len(G)
    
    # 1. Invalid short tour
    assert verify_tour_on_raw_graph([1, 2, 3], G) is False
    
    # 2. Invalid tour with duplicate vertices but correct length
    dup_tour = list(range(1, n)) + [1]
    assert verify_tour_on_raw_graph(dup_tour, G) is False
    
    # 3. Invalid tour with invalid edge (e.g. arbitrary permutation)
    arb_tour = list(range(1, n + 1))
    assert verify_tour_on_raw_graph(arb_tour, G) is False
    
    # 4. Valid small cycle on synthetic graph
    small_G = {
        1: {2, 4},
        2: {1, 3},
        3: {2, 4},
        4: {3, 1}
    }
    assert verify_tour_on_raw_graph([1, 2, 3, 4], small_G) is True
    assert verify_tour_on_raw_graph([1, 4, 3, 2], small_G) is True
    assert verify_tour_on_raw_graph([1, 3, 2, 4], small_G) is False

def test_splicer_synthetic_single_cycle():
    # Synthetic graph: 2 hubs (101, 102), 2 strips:
    # Strip 0: vertices [1, 2, 3] (path 1-2-3)
    # Strip 1: vertices [4, 5, 6] (path 4-5-6)
    # Cycle: 101 -> 1 -> 2 -> 3 -> 102 -> 6 -> 5 -> 4 -> 101
    G = collections.defaultdict(set)
    # Strip 0 internal
    G[1].add(2); G[2].add(1)
    G[2].add(3); G[3].add(2)
    # Strip 1 internal
    G[4].add(5); G[5].add(4)
    G[5].add(6); G[6].add(5)
    # Boundary edges
    G[101].add(1); G[1].add(101)
    G[3].add(102); G[102].add(3)
    G[102].add(6); G[6].add(102)
    G[4].add(101); G[101].add(4)
    
    decomp = DecompositionResult(
        all_hubs={101, 102},
        s_hubs=[101],
        b_hubs=[102],
        m_hubs=[],
        hh_edges=[],
        strips=[[1, 2, 3], [4, 5, 6]],
        strip_adj_hubs={0: {101, 102}, 1: {101, 102}},
        hub_adj_strips={101: {0, 1}, 102: {0, 1}},
        strip_hub_ports={(0, 101): [1], (0, 102): [3], (1, 101): [4], (1, 102): [6]}
    )
    
    strip_paths = {
        0: [[1, 2, 3]],
        1: [[4, 5, 6]]
    }
    
    is_valid, tour = splice_and_verify_tour(G, decomp, hh_edges=[], strip_paths=strip_paths)
    assert is_valid is True
    assert len(tour) == 8
    assert set(tour) == {1, 2, 3, 4, 5, 6, 101, 102}
    assert verify_tour_on_raw_graph(tour, G) is True

def test_splicer_detects_subtours():
    # Synthetic graph with 2 disconnected subtours:
    # Subtour 1: 101 -> 1 -> 2 -> 101
    # Subtour 2: 102 -> 3 -> 4 -> 102
    G = collections.defaultdict(set)
    G[101].add(1); G[1].add(101)
    G[1].add(2); G[2].add(1)
    G[2].add(101); G[101].add(2)
    
    G[102].add(3); G[3].add(102)
    G[3].add(4); G[4].add(3)
    G[4].add(102); G[102].add(4)
    
    decomp = DecompositionResult(
        all_hubs={101, 102},
        s_hubs=[101],
        b_hubs=[102],
        m_hubs=[],
        hh_edges=[],
        strips=[[1, 2], [3, 4]],
        strip_adj_hubs={0: {101}, 1: {102}},
        hub_adj_strips={101: {0}, 102: {1}},
        strip_hub_ports={(0, 101): [1, 2], (1, 102): [3, 4]}
    )
    
    strip_paths = {
        0: [[1, 2]],
        1: [[3, 4]]
    }
    
    is_valid, res = splice_macro_tour(G, decomp, hh_edges=[], strip_paths=strip_paths)
    assert is_valid is False
    assert len(res) == 2 # 2 subtours identified
