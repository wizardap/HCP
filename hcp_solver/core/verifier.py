"""
Rigorous mathematical verifier for Hamiltonian Cycles.
Ensures 100% soundness against raw DIMACS graphs without assumptions.
"""

from typing import Dict, List, Set, Union, Tuple
from .graph import Graph

def verify_tour(tour: List[int], G: Union[Graph, Dict[int, Set[int]]]) -> Tuple[bool, str]:
    """
    Strictly verifies whether `tour` constitutes a sound Hamiltonian cycle in `G`.
    
    Checks:
    1. Tour length matches |V| exactly.
    2. All vertices in tour are distinct (no duplicate visits).
    3. Vertex set in tour matches G's vertex set exactly.
    4. Every consecutive step in tour is a valid edge in G.
    5. The closing edge (tour[-1], tour[0]) is a valid edge in G.
    
    Returns (True, "OK") or (False, error_message).
    """
    if isinstance(G, Graph):
        adj = G.adj
        n_expected = G.num_vertices
    else:
        adj = G
        n_expected = len(adj)

    # 1. Exact vertex count
    if len(tour) != n_expected:
        return False, f"Tour length ({len(tour)}) does not match graph vertex count ({n_expected})"

    # 2. Zero duplicates
    unique_vertices = set(tour)
    if len(unique_vertices) != n_expected:
        return False, f"Tour contains duplicates ({n_expected - len(unique_vertices)} duplicated entries)"

    # 3. Matches graph vertex set
    if unique_vertices != set(adj.keys()):
        missing = set(adj.keys()) - unique_vertices
        extra = unique_vertices - set(adj.keys())
        return False, f"Tour vertices mismatch: missing {len(missing)}, extra {len(extra)}"

    # 4 & 5. Verify all edges along the cycle
    for i in range(n_expected):
        u = tour[i]
        v = tour[(i + 1) % n_expected]
        if v not in adj.get(u, set()):
            return False, f"Invalid edge at index {i} -> {(i+1)%n_expected}: ({u}, {v}) not in graph!"

    return True, "100% Mathematically Certified Hamiltonian Cycle"


def certify_tour(tour: List[int], G: Union[Graph, Dict[int, Set[int]]], graph_name: str = "") -> bool:
    """
    Convenience wrapper that verifies the tour and prints an authoritative certification banner.
    Raises ValueError if the tour is invalid.
    """
    valid, msg = verify_tour(tour, G)
    n = len(tour)
    tag = f" FOR {graph_name}" if graph_name else ""
    if not valid:
        print(f"[-] MATHEMATICAL VERIFICATION FAILED{tag}: {msg}")
        raise ValueError(f"Tour verification failed: {msg}")

    print("+" + "=" * 70 + "+")
    print(f"| [VERIFIED] HAMILTONIAN TOUR CONFIRMED{tag:<30} |")
    print(f"| Vertices: {n:<10} Duplicates: 0{' ':<24} Edges: {n:<12} |")
    print(f"| 100% sound: all {n} edges verified against raw DIMACS graph.      |")
    print("+" + "=" * 70 + "+")
    return True
