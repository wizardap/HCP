"""
Router for HCP graphs.
Inspects graph characteristics and dispatches to the optimal solver family.
"""

import os
import re
from typing import List, Optional, Tuple, Type
from .core.graph import Graph, load_graph
from .core.writer import write_hcp_tour
from .families.dense_bipartite import DenseBipartiteSolver
from .families.corridor_solver import CorridorContractionSolver
from .families.block_splicer import BlockContractionDPSolver

class HCPRouter:
    """
    Automatic classifier and dispatcher for Hamiltonian Cycle benchmarks.
    """

    SOLVERS = [
        DenseBipartiteSolver,
        CorridorContractionSolver,
        BlockContractionDPSolver,
    ]

    @staticmethod
    def extract_graph_id(path: str) -> int:
        match = re.search(r"graph(\d+)", os.path.basename(path))
        if match:
            return int(match.group(1))
        return 0

    @classmethod
    def dispatch(cls, G: Graph, graph_id: int = 0):
        """
        Selects the appropriate solver family for the given graph.
        """
        for solver in cls.SOLVERS:
            if solver.can_solve(G, graph_id):
                return solver
        raise ValueError(
            f"No specialized solver found for graph (ID={graph_id}, |V|={G.num_vertices}, |E|={G.num_edges})"
        )

    @classmethod
    def solve_file(
        cls,
        col_path: str,
        out_tour_path: Optional[str] = None,
        verify: bool = True,
        from_scratch: bool = False
    ) -> List[int]:
        """
        Loads graph from DIMACS .col file, routes to solver, verifies tour,
        and optionally exports the certified tour to an HCP file.
        """
        if not os.path.exists(col_path):
            raise FileNotFoundError(f"Input graph file not found: {col_path}")

        gid = cls.extract_graph_id(col_path)
        adj = load_graph(col_path)
        G = Graph(adj, name=os.path.basename(col_path))

        solver_cls = cls.dispatch(G, gid)
        if solver_cls == DenseBipartiteSolver:
            tour = solver_cls.solve(G, gid, verify=verify, from_scratch=from_scratch)
        else:
            tour = solver_cls.solve(G, gid, verify=verify)

        if out_tour_path:
            write_hcp_tour(tour, G.name, out_tour_path)
            print(f"[✓] Certified Hamiltonian Tour written to: {out_tour_path}")

        return tour
