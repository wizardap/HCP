"""
HCP Solver: A lean, unified, and router-free solver for Hamiltonian Cycle Problem benchmarks.
Provides pure algorithmic solving for both general graphs and challenge benchmarks.
"""

from .core.pipeline import solve_general_hcp
from .core.graph import Graph, load_graph
from .core.verifier import verify_tour

__version__ = "1.0.0"
__all__ = ["solve_general_hcp", "Graph", "load_graph", "verify_tour"]
