import os
from typing import Dict, List, Set, Tuple

def assemble_full_tour(comp0_cycle: List[int], corridors: List[Dict]) -> List[int]:
    """
    Slices all corridor paths into the Comp 0 Hamiltonian cycle in place of their virtual edges.
    """
    virt_map = {}
    for c in corridors:
        eu, ev = c['ext_ports']
        key = frozenset([eu, ev])
        assert key not in virt_map, f"Duplicate virtual edge {key}"
        virt_map[key] = c

    n = len(comp0_cycle)
    tour = []
    visited_virts = set()

    for i in range(n):
        x = comp0_cycle[i]
        y = comp0_cycle[(i + 1) % n]
        tour.append(x)

        key = frozenset([x, y])
        if key in virt_map:
            assert key not in visited_virts, f"Virtual edge {key} visited multiple times!"
            visited_virts.add(key)
            corr = virt_map[key]
            u, v = corr['ports']
            eu, ev = corr['ext_ports']
            path = corr['path']

            if x == eu:
                # Traverse eu -> u ... v -> ev
                c_oriented = path if path[0] == u else list(reversed(path))
            else:
                # Traverse ev -> v ... u -> eu
                c_oriented = path if path[0] == v else list(reversed(path))
            tour.extend(c_oriented)

    assert len(visited_virts) == len(corridors), f"Expected {len(corridors)} virtual edges, spliced {len(visited_virts)}"
    assert len(set(tour)) == len(tour), "Tour has duplicate vertices!"
    return tour

def export_hcp_tour(tour: List[int], graph_name: str, output_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w") as f:
        f.write(f"NAME : {graph_name}.tour\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write(f"TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")
    print(f"[✓] Tour exported to {output_path}")
