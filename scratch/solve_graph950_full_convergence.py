import time, sys, os, collections
from typing import Dict, List, Set, Tuple, Any

from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.pinpointed_strip_solver import PinpointedStripSolver
from scratch.graph950.global_demand_coordinator import GlobalDemandCoordinator
from scratch.graph950.macro_splicer import splice_macro_tour, verify_tour_on_raw_graph
from scratch.graph950.two_tier_orchestrator import write_hcp_tour
from pysat.solvers import Cadical195

def precompute_tiny_invalid_pairs(G, decomp):
    invalid_pairs = []
    tiny_strips = [si for si in range(len(decomp.strips)) if len(decomp.strips[si]) < 20]
    for si in tiny_strips:
        s = decomp.strips[si]
        s_set = set(s)
        hubs = sorted(list(decomp.strip_adj_hubs[si]))
        for i in range(len(hubs)):
            for j in range(i+1, len(hubs)):
                h1, h2 = hubs[i], hubs[j]
                active_edges = [(u, w) for u in s for w in G[u] if w in s_set and u < w]
                for u in s:
                    if h1 in G[u]: active_edges.append((min(u, h1), max(u, h1)))
                    if h2 in G[u]: active_edges.append((min(u, h2), max(u, h2)))
                active_edges = list(set(active_edges))
                e2v = {e: idx+1 for idx, e in enumerate(active_edges)}
                clauses = []
                adj = collections.defaultdict(list)
                for u, w in active_edges:
                    adj[u].append(e2v[(u, w)]); adj[w].append(e2v[(u, w)])
                for u in s:
                    inc = adj[u]
                    if len(inc) < 2: clauses.append([]); break
                    if len(inc) == 2: clauses.append([inc[0]]); clauses.append([inc[1]])
                    else:
                        for idx in range(len(inc)): clauses.append([inc[m] for m in range(len(inc)) if m != idx])
                        for m1 in range(len(inc)):
                            for m2 in range(m1+1, len(inc)):
                                for m3 in range(m2+1, len(inc)): clauses.append([-inc[m1], -inc[m2], -inc[m3]])
                for h in [h1, h2]:
                    inc = adj[h]
                    if not inc: clauses.append([]); break
                    clauses.append(inc)
                    for m1 in range(len(inc)):
                        for m2 in range(m1+1, len(inc)): clauses.append([-inc[m1], -inc[m2]])
                is_valid = False
                if [] not in clauses:
                    with Cadical195(bootstrap_with=clauses) as sol:
                        if sol.solve(): is_valid = True
                if not is_valid:
                    invalid_pairs.append((si, h1, h2))
    return invalid_pairs

def solve_graph950_converge(graph_path="FHCPCS-col/graph950.col", timeout=600.0, output_path="scratch/graph950_perfect_certified.hcp"):
    t_start = time.time()
    print("========================================================================")
    print("FULL CONVERGENCE TWO-TIER CDCL SOLVER ON graph950 (N = 6620)...")
    print("========================================================================")
    
    G, degs = load_graph(graph_path)
    decomp = decompose_graph(G, degs)
    print(f"Decomposition: {len(decomp.all_hubs)} Hubs, {len(decomp.strips)} Strips, {len(decomp.hh_edges)} HH edges.")
    
    t_pre = time.time()
    invalid_pairs = precompute_tiny_invalid_pairs(G, decomp)
    print(f"Pre-computed and forbidden {len(invalid_pairs)} invalid hub-pairs in {time.time()-t_pre:.2f}s.")
    
    coordinator = GlobalDemandCoordinator(G, decomp)
    for si, h1, h2 in invalid_pairs:
        v1 = coordinator.var_d1.get((si, h1))
        v2 = coordinator.var_d1.get((si, h2))
        if v1 and v2:
            coordinator.solver.add_clause([-v1, -v2])
            
    strip_solver = PinpointedStripSolver(G, decomp)
    
    outer_it = 0
    while True:
        outer_it += 1
        elapsed = time.time() - t_start
        if elapsed > timeout:
            print(f"[TIMEOUT] Reached global {timeout}s limit at iteration {outer_it}")
            return False
            
        is_sat, hh_edges, strip_demands = coordinator.solve_assignment()
        if not is_sat:
            print(f"Coordinator returned UNSAT at iteration {outer_it}: Search space exhausted.")
            return False
            
        all_strips_sat = True
        strip_paths = {}
        
        for si, s in enumerate(decomp.strips):
            if time.time() - t_start > timeout:
                print(f"[TIMEOUT] Global timeout reached during strip solving.")
                return False
                
            s_hub = list(decomp.strip_adj_hubs[si] & set(decomp.s_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.s_hubs)) else None
            b_hub = list(decomp.strip_adj_hubs[si] & set(decomp.b_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.b_hubs)) else None
            dem = strip_demands.get(si, {})
            
            tot_d = sum(dem.values())
            K = tot_d // 2 if tot_d >= 2 else (1 if len(s) < 10 else 4)
            
            sat, res = strip_solver.solve_strip(si, dem, s_hub, b_hub, K=K)
            if not sat:
                all_strips_sat = False
                failed_core = res if isinstance(res, list) else list(dem.keys())
                coordinator.add_conflict_clause(si, dem, failed_core)
                if outer_it % 10 == 1 or outer_it <= 5:
                    print(f"  Iter {outer_it:3d} ({elapsed:.1f}s): Strip {si:2d} ({len(s)}v) UNSAT -> Conflict learned")
                break
            else:
                strip_paths[si] = res
                
        if not all_strips_sat:
            continue
            
        print(f"\n===> Iter {outer_it:3d} ({time.time()-t_start:.1f}s): ALL 74 STRIPS SATISFIED! Splicing tour...")
        is_valid, res = splice_macro_tour(G, decomp, hh_edges, strip_paths, strip_demands)
        
        if is_valid:
            tour = res
            print("\n" + "="*72)
            print(f"===> *** 100% CERTIFIED SINGLE HAMILTONIAN TOUR FOUND FOR graph950! ***")
            print(f"     Tour Length: {len(tour)}/6620")
            print(f"     Unique Vertices: {len(set(tour))}/6620")
            print(f"     Total End-to-End Time: {time.time()-t_start:.2f}s")
            print("="*72)
            
            if verify_tour_on_raw_graph(tour, G):
                print("STANDALONE INDEPENDENT CERTIFICATION: PASSED (100% Valid)")
                write_hcp_tour(tour, output_path)
                print(f"Exported certified tour to: {output_path}")
                return True
            else:
                print("ERROR: Raw verification failed.")
                return False
        else:
            subtours = res
            print(f"     Splicer found {len(subtours)} subtours. Adding macro cuts...")
            for cyc in subtours:
                coordinator.add_macro_cut(set(cyc), hh_edges, strip_demands)

if __name__ == '__main__':
    solve_graph950_converge()
