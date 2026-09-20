"""
Router compatibility layer for HCP graphs.
Redirects to the universal, router-free GeneralHcpEngine.
"""

import os
from typing import List, Optional

from .core.graph import Graph, load_graph
from .core.pipeline import solve_general_hcp
from .core.writer import write_hcp_tour

class HCPRouter:
    """
    Deprecated: Provided for backward compatibility.
    Redirects calls directly to the universal router-free GeneralHcpEngine.
    """

    @classmethod
    def solve_file(
        cls,
        col_path: str,
        out_tour_path: Optional[str] = None,
        verify: bool = True,
        from_scratch: bool = True
    ) -> List[int]:
        if not os.path.exists(col_path):
            raise FileNotFoundError(f"Input graph file not found: {col_path}")

        tour = solve_general_hcp(
            col_path_or_adj=col_path,
            verbose=True,
            from_scratch=from_scratch
        )

        if out_tour_path:
            name = os.path.basename(col_path)
            write_hcp_tour(tour, name, out_tour_path)
            print(f"[✓] Certified Hamiltonian Tour written to: {out_tour_path}")

        return tour
