import os, pytest
from scratch.graph677.solve_graph677 import assemble_tour

def test_assemble_tour_toy():
    # Toy example: Comp 0 cycle = [1, 2, 3, 4] with virtual edge (2, 3)
    # Module path = [10, 20, 30], port1=10 (connects to 2), port2=30 (connects to 3)
    comp0_cycle = [1, 2, 3, 4]
    mod_path = [10, 20, 30]
    tour = assemble_tour(comp0_cycle, mod_path, port1=10, port2=30, p1_ext=2, p2_ext=3, expected_len=7)
    assert len(tour) == 7
    assert tour == [1, 2, 10, 20, 30, 3, 4]
