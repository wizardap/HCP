"""
TSPLIB / HCP Tour File Writer.
"""

import os
from typing import List

def write_hcp_tour(tour: List[int], graph_name: str, output_path: str) -> str:
    """
    Writes a Hamiltonian tour in standard TSPLIB TOUR format:
    NAME : <graph_name>
    TYPE : TOUR
    DIMENSION : <N>
    TOUR_SECTION
    <v_1>
    ...
    <v_N>
    -1
    EOF
    """
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(f"NAME : {graph_name}\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for node in tour:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")
    return output_path
