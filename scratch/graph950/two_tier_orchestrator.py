import argparse
import collections
import os
import sys
import time
from typing import Dict, List, Set, Tuple, Any, Optional

# Ensure project root is in sys.path
_repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if _repo_root not in sys.path:
    sys.path.insert(0, _repo_root)

from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph, DecompositionResult
from scratch.graph950.pinpointed_strip_solver import PinpointedStripSolver
from scratch.graph950.global_demand_coordinator import GlobalDemandCoordinator
from scratch.graph950.macro_splicer import splice_and_verify_tour, splice_macro_tour, verify_tour_on_raw_graph
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType


def write_hcp_tour(tour: List[int], output_path: str, graph_name: str = "graph950"):
    """Writes a tour in standard TSPLIB/HCP tour format."""
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w") as f:
        f.write(f"NAME : {graph_name}\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")


def solve_graph950_two_tier(
    graph_path: str = "FHCPCS-col/graph950.col",
    timeout: float = 1800.0,
    dry_run: bool = False,
    output_path: str = "scratch/graph950/found_tour_puresat.hcp"
) -> bool:
    """
    End-to-End Two-Tier Demand-Coordinated Solver for graph950.col.
    Integrates two-tier decomposition, global demand coordinator,
    pinpointed strip solver with flexible K in {2, 3, 4, 5}, and
    macro splicing with Cut-Block CEGAR loop.
    """
    t_start = time.time()
    print(f"=== Starting Two-Tier Demand-Coordinated Solver on {graph_path} ===")
    
    if not os.path.exists(graph_path):
        print(f"Error: graph file {graph_path} not found.")
        return False
        
    G, degs = load_graph(graph_path)
    decomp = decompose_graph(G, degs)
    print(f"Decomposition: {len(decomp.all_hubs)} hubs ({len(decomp.s_hubs)} S, {len(decomp.b_hubs)} B, {len(decomp.m_hubs)} M), {len(decomp.strips)} strips")
    
    if dry_run:
        print("[DRY RUN] Initialized cleanly.")
        return True

    strip_solver = PinpointedStripSolver(G, decomp)
    coordinator = GlobalDemandCoordinator(G, decomp)

    # Determine supported K preference per strip
    k_preferred = {}
    for si, s in enumerate(decomp.strips):
        if len(s) <= 10:
            k_preferred[si] = 1
        else:
            s_hub = list(decomp.strip_adj_hubs[si] & set(decomp.s_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.s_hubs)) else None
            b_hub = list(decomp.strip_adj_hubs[si] & set(decomp.b_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.b_hubs)) else None
            for k in [4, 3, 5, 2]:
                sat, res = strip_solver.solve_strip(si, {}, s_hub, b_hub, K=k)
                if sat:
                    k_preferred[si] = k
                    break
            if si not in k_preferred:
                k_preferred[si] = 2

    print(f"Computed Strip K Preferences: {collections.Counter(k_preferred.values())}")

    outer_it = 0
    while True:
        outer_it += 1
        elapsed = time.time() - t_start
        if elapsed > timeout:
            print(f"[TIMEOUT] Reached global {timeout}s limit at iteration {outer_it}")
            return False

        print(f"\n--- Outer Iteration {outer_it} ({elapsed:.1f}s) ---")
        is_sat, hh_edges, strip_demands = coordinator.solve_assignment()
        if not is_sat:
            print("Coordinator returned UNSAT: search space exhausted.")
            return False

        print(f"Coordinator assigned {len(hh_edges)} Hub-Hub edges across {len(decomp.strips)} strips")

        all_strips_sat = True
        strip_paths = {}

        for si, s in enumerate(decomp.strips):
            if time.time() - t_start > timeout:
                print(f"[TIMEOUT] Global timeout reached during strip solving.")
                return False

            s_hub = list(decomp.strip_adj_hubs[si] & set(decomp.s_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.s_hubs)) else None
            b_hub = list(decomp.strip_adj_hubs[si] & set(decomp.b_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.b_hubs)) else None
            m_dem = strip_demands.get(si, {})

            # Flexible K selection in {2, 3, 4, 5} for large strips or 1 for small strips
            if len(s) <= 10:
                k_candidates = [1]
            else:
                pref = k_preferred.get(si, 4)
                other_k = [k for k in [4, 3, 2, 5] if k != pref]
                k_candidates = [pref] + other_k

            sat = False
            res = None
            for k in k_candidates:
                sat, res = strip_solver.solve_strip(si, m_dem, s_hub, b_hub, K=k)
                if sat:
                    break

            if not sat:
                all_strips_sat = False
                failed_core = res if isinstance(res, list) else list(m_dem.keys())
                coordinator.add_conflict_clause(si, failed_core)
                print(f"  Strip {si:2d} ({len(s)}v) UNSAT with core {failed_core} -> conflict learned")
                break
            else:
                strip_paths[si] = res

        if not all_strips_sat:
            continue

        print(f"All {len(decomp.strips)} strips SATISFIED! Splicing full tour...")
        is_valid, res = splice_macro_tour(G, decomp, hh_edges, strip_paths)

        if is_valid:
            tour = res
            print(f"SUCCESS! Single Hamiltonian tour formed with {len(tour)} vertices!")
            if verify_tour_on_raw_graph(tour, G):
                print(f"CERTIFICATION PASSED: Verified tour independently on raw graph G!")
                write_hcp_tour(tour, output_path)
                print(f"Wrote certified tour to {output_path}")
                print(f"Total time elapsed: {time.time() - t_start:.2f}s")
                return True
            else:
                print("Verification on raw graph failed.")
                return False
        else:
            subtours = res
            print(f"Splicer detected {len(subtours)} disconnected subtours -> adding macro cut clauses")
            for cyc in subtours:
                cyc_hubs = set(cyc) & decomp.all_hubs
                if 0 < len(cyc_hubs) < len(decomp.all_hubs):
                    coordinator.add_macro_cut(cyc_hubs)


def main():
    parser = argparse.ArgumentParser(description="End-to-End Two-Tier Demand Coordinator Solver")
    parser.add_argument("--graph", type=str, default="FHCPCS-col/graph950.col", help="Path to .col graph file")
    parser.add_argument("--timeout", type=float, default=1800.0, help="Wall-clock timeout in seconds")
    parser.add_argument("--out", type=str, default="scratch/graph950/found_tour_puresat.hcp", help="Output path for .hcp tour")
    parser.add_argument("--dry_run", action="store_true", help="Perform dry run initialization without solving")
    args = parser.parse_args()

    success = solve_graph950_two_tier(
        graph_path=args.graph,
        timeout=args.timeout,
        dry_run=args.dry_run,
        output_path=args.out
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
