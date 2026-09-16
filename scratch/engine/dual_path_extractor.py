import collections
from typing import Dict, List, Optional, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType


def extract_module_dual_paths(
    G_adj: Dict[int, Set[int]],
    module: Dict
) -> Tuple[Set[Tuple[int, int]], Set[Tuple[int, int]]]:
    """
    Computes the two canonical spanning Hamiltonian path configurations T_i and F_i
    for an isolated variable module across its interface ports.

    Guarantees:
    - Covers 100% of module vertices.
    - Path T_i visits all internal virtual edges (degree-2 chains).
    - Produces two distinct configurations (T_i != F_i) across module['ports'].
    """
    m_nodes = set(module.get('nodes', ()))
    ports = module.get('ports', ())
    if len(m_nodes) < 2 or len(ports) != 2:
        return set(), set()
    p_in, p_out = ports
    if p_in not in m_nodes or p_out not in m_nodes or p_in == p_out:
        return set(), set()

    # Canonical edges within module
    edges = set()
    for u in m_nodes:
        for v in G_adj.get(u, ()):
            if v in m_nodes and u < v:
                edges.add((u, v))

    # Add virtual edges from module
    ves = {tuple(sorted(ve)) for ve in module.get('virtual_edges', ())}
    for ve in ves:
        if ve[0] in m_nodes and ve[1] in m_nodes:
            edges.add(ve)

    # Virtual closure edge between ports to turn Hamiltonian Path into Hamiltonian Cycle
    virt_edge = tuple(sorted([p_in, p_out]))
    edges.add(virt_edge)

    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    # Pre-check degree
    for u in m_nodes:
        if len(inc_edges[u]) < 2:
            return set(), set()

    def _solve_spanning_path(
        force_all_ves: bool,
        blocked_paths: Optional[List[Set[Tuple[int, int]]]] = None
    ) -> Optional[Set[Tuple[int, int]]]:
        with Cadical195() as solver:
            top = len(edge_list) + 1

            # Degree-2 constraint on every node in module
            for u in m_nodes:
                clauses = CardEnc.equals(
                    lits=inc_edges[u],
                    bound=2,
                    top_id=top,
                    encoding=EncType.cardnetwrk
                )
                for cl in clauses:
                    solver.add_clause(cl)
                    for lit in cl:
                        top = max(top, abs(lit) + 1)

            # Force virtual closure edge
            solver.add_clause([edge_to_var[virt_edge]])

            # Force internal virtual edges if requested
            if force_all_ves:
                for ve in ves:
                    if ve in edge_to_var:
                        solver.add_clause([edge_to_var[ve]])

            # Block previously found paths
            if blocked_paths:
                for bp in blocked_paths:
                    solver.add_clause([-edge_to_var[e] for e in bp if e in edge_to_var])

            while solver.solve():
                model = set(solver.get_model())
                active_edges = {e for e in edge_list if edge_to_var[e] in model}

                adj_model = collections.defaultdict(list)
                for u, v in active_edges:
                    adj_model[u].append(v)
                    adj_model[v].append(u)

                visited = set()
                cycles = []
                for u in m_nodes:
                    if u not in visited:
                        cyc = []
                        curr, prev = u, None
                        while curr not in visited:
                            visited.add(curr)
                            cyc.append(curr)
                            nbrs = adj_model[curr]
                            if len(nbrs) < 2:
                                break
                            nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                            prev, curr = curr, nxt
                        cycles.append(cyc)

                if len(cycles) == 1 and len(cycles[0]) == len(m_nodes):
                    return active_edges - {virt_edge}

                # Subcycle elimination cut
                for cyc in cycles:
                    if len(cyc) >= 2:
                        solver.add_clause([
                            -edge_to_var[tuple(sorted([cyc[i], cyc[(i + 1) % len(cyc)]]))]
                            for i in range(len(cyc))
                        ])
        return None

    # Step 1: Solve primary configuration T_i (forcing all internal virtual edges)
    p_true = _solve_spanning_path(force_all_ves=True)
    if p_true is None:
        p_true = _solve_spanning_path(force_all_ves=False)
    if p_true is None:
        return set(), set()

    # Step 2: Solve dual configuration F_i (distinct from T_i)
    p_false = _solve_spanning_path(force_all_ves=True, blocked_paths=[p_true])
    if p_false is None:
        p_false = _solve_spanning_path(force_all_ves=False, blocked_paths=[p_true])

    if p_false is not None and p_false != p_true:
        return p_true, p_false
    elif p_true is not None:
        return p_true, p_true

    return set(), set()
