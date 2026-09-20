"""
General, Router-Free HCP Solver Pipeline.
Takes an arbitrary graph G = (V, E) without any knowledge of graph ID or filename.

Unified 5-stage automated structural reduction and CEGAR pipeline:
1. Automated Macro-Decomposition Check (for dense bipartite / bridge corridor / block DP).
2. Automated Degree-2 Chain Contraction.
3. High-Performance Core SAT-CEGAR Engine (Rust `cegar-fix` release binary).
4. Pure Sound CDCL PySAT-CEGAR Fallback (CaDiCaL with strict DFJ subcycle cuts).
5. Tour Assembly & Strict Independent Soundness Certification.
"""

import collections
import os
import subprocess
import sys
import tempfile
import time
from typing import Dict, List, Optional, Set, Tuple, Union

from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

from .graph import Graph, load_graph
from .verifier import verify_tour
from .contraction import contract_degree2_chains, expand_contracted_cycle

# Locate the compiled Rust cegar-fix binary
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CANDIDATE_BIN_PATHS = [
    os.path.join(REPO_ROOT, "src", "cegar-fix", "target", "release", "cegar-fix"),
    os.path.join(REPO_ROOT, "bin", "cegar-fix"),
]

def get_cegar_binary_path() -> Optional[str]:
    for p in CANDIDATE_BIN_PATHS:
        if os.path.isfile(p) and os.access(p, os.X_OK):
            return p
    return None

def write_temp_col(adj: Dict[int, Set[int]]) -> str:
    """Writes an adjacency dict to a temporary DIMACS .col file."""
    fd, tmp_path = tempfile.mkstemp(suffix=".col", prefix="hcp_temp_")
    with os.fdopen(fd, "w") as f:
        edges = []
        for u in adj:
            for v in adj[u]:
                if u < v:
                    edges.append((u, v))
        f.write(f"p edge {len(adj)} {len(edges)}\n")
        for u, v in edges:
            f.write(f"e {u} {v}\n")
    return tmp_path

def parse_cegar_output(stdout: str) -> Optional[List[int]]:
    """Parses tour from cegar-fix stdout."""
    lines = stdout.splitlines()
    for i, line in enumerate(lines):
        if line.strip() == "solution:" and i + 1 < len(lines):
            tour_line = lines[i + 1].strip()
            if tour_line:
                try:
                    tour = [int(x) for x in tour_line.split()]
                    if len(tour) >= 3:
                        return tour
                except ValueError:
                    pass
    return None

def solve_core_sat_binary(
    col_path: str,
    timeout_sec: float = 300.0,
    verbose: bool = False
) -> Optional[List[int]]:
    """Runs compiled cegar-fix Rust solver on a .col file."""
    bin_path = get_cegar_binary_path()
    if not bin_path:
        return None

    cmd = [
        bin_path,
        "-i", col_path,
        "--auto", "1",
        "--timeout", str(int(timeout_sec))
    ]

    try:
        res = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout_sec + 5.0
        )
        if "s SATISFIABLE" in res.stdout:
            tour = parse_cegar_output(res.stdout)
            return tour
    except subprocess.TimeoutExpired:
        if verbose:
            print("[*] Rust cegar-fix timed out.")
    except Exception as e:
        if verbose:
            print(f"[*] Warning: Rust binary failed ({e})")
    return None

def solve_core_sat_pysat(
    adj: Dict[int, Set[int]],
    timeout_sec: float = 300.0,
    verbose: bool = False,
    forced_edges: Optional[Set[Tuple[int, int]]] = None
) -> Optional[List[int]]:
    """
    Pure Python 100% sound CDCL SAT-CEGAR solver with DFJ cuts.
    Fallback engine when Rust binary is unavailable.
    """
    t0 = time.time()
    verts = sorted(list(adj.keys()))
    nv = len(verts)
    if nv < 3:
        return None

    edges = []
    for u in verts:
        for v in adj[u]:
            if u < v:
                edges.append((u, v))

    var_e = {e: i + 1 for i, e in enumerate(edges)}
    for u, v in edges:
        var_e[(v, u)] = var_e[(u, v)]

    clauses = []
    top = len(edges)
    for u in verts:
        inc = [var_e[(u, v)] for v in adj[u]]
        cnf = CardEnc.equals(lits=inc, bound=2, top_id=top, encoding=EncType.seqcounter)
        top = max(top, cnf.nv)
        clauses.extend(cnf.clauses)

    if forced_edges:
        for fe in forced_edges:
            e = tuple(sorted(fe))
            if e in var_e:
                clauses.append([var_e[e]])

    solver = Cadical195(bootstrap_with=clauses)
    it = 0

    while time.time() - t0 < timeout_sec:
        it += 1
        sat = solver.solve()
        if not sat:
            solver.delete()
            return None

        model = set(solver.get_model())
        active_adj = collections.defaultdict(list)
        for u, v in edges:
            if var_e[(u, v)] in model:
                active_adj[u].append(v)
                active_adj[v].append(u)

        vis = set()
        cycles = []
        for u in verts:
            if u not in vis:
                cyc = [u]
                vis.add(u)
                curr = u
                prev = None
                while True:
                    nxts = [w for w in active_adj[curr] if w != prev]
                    if not nxts or nxts[0] == u:
                        break
                    nxt = nxts[0]
                    cyc.append(nxt)
                    vis.add(nxt)
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1 and len(cycles[0]) == nv:
            solver.delete()
            if verbose:
                print(f"[*] PySAT-CEGAR converged in {it} iters ({time.time()-t0:.2f}s)")
            return cycles[0]

        if 2 <= len(cycles) <= 4:
            forbidden_delete = set(tuple(sorted(e)) for e in (forced_edges or []))
            merged_tour = _try_fast_2opt_merge(cycles, adj, forbidden_delete)
            if merged_tour is not None and len(merged_tour) == nv:
                solver.delete()
                if verbose:
                    print(f"[*] 2-opt merged full tour in {it} iters ({time.time()-t0:.2f}s)")
                return merged_tour

        # Add sound DFJ Subcycle Elimination Cuts
        for cyc in cycles:
            cyc_set = set(cyc)
            cut_edges = [var_e[(u, v)] for u in cyc for v in adj[u] if v not in cyc_set]
            if cut_edges:
                solver.add_clause(cut_edges)

    solver.delete()
    return None

def _try_fast_2opt_merge(
    cycles: List[List[int]],
    adj: Dict[int, Set[int]],
    forbidden_delete: Set[Tuple[int, int]] = frozenset()
) -> Optional[List[int]]:
    curr = list(cycles)
    merged_any = True
    while merged_any and len(curr) > 1:
        merged_any = False
        curr.sort(key=len, reverse=True)
        for i in range(len(curr)):
            for j in range(i + 1, len(curr)):
                res = _merge_two_cycles(curr[i], curr[j], adj, forbidden_delete)
                if res is not None:
                    curr.pop(j)
                    curr[i] = res
                    merged_any = True
                    break
            if merged_any:
                break
    if len(curr) == 1:
        return curr[0]
    return None

def _merge_two_cycles(
    c1: List[int],
    c2: List[int],
    adj: Dict[int, Set[int]],
    forbidden_delete: Set[Tuple[int, int]] = frozenset()
) -> Optional[List[int]]:
    n1, n2 = len(c1), len(c2)
    for i in range(n1):
        u1 = c1[i]
        u2 = c1[(i + 1) % n1]
        e1 = tuple(sorted([u1, u2]))
        if e1 in forbidden_delete:
            continue
        for j in range(n2):
            v1 = c2[j]
            v2 = c2[(j + 1) % n2]
            e2 = tuple(sorted([v1, v2]))
            if e2 in forbidden_delete:
                continue
            if v1 in adj[u1] and v2 in adj[u2]:
                tour = []
                for k in range(1, n1 + 1):
                    tour.append(c1[(i + k) % n1])
                for k in range(n2):
                    tour.append(c2[(j + n2 - (k % n2)) % n2])
                return tour
            if v2 in adj[u1] and v1 in adj[u2]:
                tour = []
                for k in range(1, n1 + 1):
                    tour.append(c1[(i + k) % n1])
                for k in range(n2):
                    tour.append(c2[(j + 1 + k) % n2])
                return tour
    return None

def solve_general_hcp(
    col_path_or_adj: Union[str, Dict[int, Set[int]]],
    timeout_sec: float = 300.0,
    verbose: bool = True,
    from_scratch: bool = True
) -> List[int]:
    """
    Universal, 100% Router-Free General Solver for ANY Hamiltonian Cycle Problem instance.
    Accepts DIMACS .col file path or adjacency dictionary.
    Returns certified Hamiltonian tour as List[int].
    """
    t_start = time.time()
    temp_col_file = None

    if isinstance(col_path_or_adj, str):
        col_file = col_path_or_adj
        adj = load_graph(col_path_or_adj)
        name = os.path.basename(col_path_or_adj)
    elif isinstance(col_path_or_adj, dict):
        adj = col_path_or_adj
        name = "in_memory_graph"
        temp_col_file = write_temp_col(adj)
        col_file = temp_col_file
    else:
        raise TypeError("Input must be file path (.col) or Dict[int, Set[int]]")

    try:
        G = Graph(adj, name=name)
        N = G.num_vertices
        M = G.num_edges

        if verbose:
            print(f"[*] Universal HCP Engine: Solving '{name}' (|V|={N}, |E|={M})...")

        # Stage 1: Macro-Decomposition Check (for massive challenge instances |V| >= 4000)
        if N >= 4000:
            from ..families.dense_bipartite import DenseBipartiteSolver
            from ..families.corridor_solver import CorridorContractionSolver
            from ..families.block_splicer import BlockContractionDPSolver

            macro_solver = None
            if DenseBipartiteSolver.can_solve(G):
                macro_solver = DenseBipartiteSolver
            elif BlockContractionDPSolver.can_solve(G):
                macro_solver = BlockContractionDPSolver
            elif CorridorContractionSolver.can_solve(G):
                macro_solver = CorridorContractionSolver

            if macro_solver is not None:
                if verbose:
                    print(f"[*] Stage 1: Macro-topology matched {macro_solver.__name__}.")
                try:
                    tour = macro_solver.solve(G, from_scratch=from_scratch, verify=False)
                    ok, msg = verify_tour(tour, G)
                    if ok:
                        if verbose:
                            elapsed = time.time() - t_start
                            print(f"[✓] Successfully solved via {macro_solver.__name__} in {elapsed:.3f}s!")
                        return tour
                except Exception as e:
                    if verbose:
                        print(f"[*] Macro-solver fallback to core CEGAR: {e}")

        # Stage 2: Direct High-Performance Rust cegar-fix execution
        rem_timeout = max(5.0, timeout_sec - (time.time() - t_start))
        tour = solve_core_sat_binary(col_file, timeout_sec=rem_timeout, verbose=verbose)
        if tour is not None:
            ok, msg = verify_tour(tour, G)
            if ok:
                if verbose:
                    elapsed = time.time() - t_start
                    print(f"[✓] Tour 100% SOUND & Certified in {elapsed:.3f}s! (Dimension: {len(tour)} vertices)")
                return tour

        # Stage 3: Automated Degree-2 Chain Contraction + Python PySAT-CEGAR Fallback
        if verbose:
            print(f"[*] Stage 3: Running Degree-2 Chain Contraction & PySAT-CEGAR fallback...")

        rem_nodes, contracted_adj, chain_map = contract_degree2_chains(G.adj, set(G.adj.keys()), set())
        N_c = len(contracted_adj)
        if N_c < N and verbose:
            print(f"[*] Contracted {N - N_c} degree-2 vertices ({N} -> {N_c} vertices).")

        rem_timeout = max(5.0, timeout_sec - (time.time() - t_start))
        core_tour = solve_core_sat_pysat(
            contracted_adj,
            timeout_sec=rem_timeout,
            verbose=verbose,
            forced_edges=set(chain_map.keys())
        )
        if not core_tour:
            raise RuntimeError(f"Solver timed out or failed on graph '{name}' (|V|={N})")

        full_tour = expand_contracted_cycle(core_tour, chain_map) if chain_map else core_tour

        ok, msg = verify_tour(full_tour, G)
        if not ok:
            raise ValueError(f"Verification failed on produced tour: {msg}")

        if verbose:
            elapsed = time.time() - t_start
            print(f"[✓] Tour 100% SOUND & Certified in {elapsed:.3f}s! (Dimension: {len(full_tour)} vertices)")

        return full_tour

    finally:
        if temp_col_file and os.path.exists(temp_col_file):
            try:
                os.remove(temp_col_file)
            except OSError:
                pass
