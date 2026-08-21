import collections
from typing import Dict, List, Set, Tuple, Any, Optional
from pysat.solvers import Cadical195


class PinpointedStripSolver:
    def __init__(self, G: Dict[int, Set[int]], decomp: Any):
        self.G = G
        self.decomp = decomp

    def solve_strip(
        self,
        si: int,
        m_demands: Dict[int, int],
        s_hub: int,
        b_hub: int,
        K: int = 4,
        timeout_sec: float = 5.0
    ) -> Tuple[bool, Any]:
        verts = self.decomp.strips[si]
        v_set = set(verts)
        n = len(verts)

        if n < K:
            return False, list(m_demands.keys())

        # Build Strip Internal CNF
        # Variables: var_e[(u, v)] for internal edges
        # var_end[u]: true if u is an endpoint (deg_in_cover == 1)
        nv = 0
        var_e = {}
        for u in verts:
            for v in self.G[u]:
                if v in v_set and u < v:
                    nv += 1
                    var_e[(u, v)] = nv
                    var_e[(v, u)] = nv

        var_end = {}
        for u in verts:
            nv += 1
            var_end[u] = nv

        C = []

        # 1. Degree constraints in cover: deg in {1, 2}
        for u in verts:
            inc = [var_e[(u, v)] for v in self.G[u] if v in v_set]
            if not inc:
                return False, ("isolated_vertex", u)

            # deg >= 1
            C.append(inc)

            # deg <= 2: for all triples of incident edges
            for i in range(len(inc)):
                for j in range(i + 1, len(inc)):
                    for k in range(j + 1, len(inc)):
                        C.append([-inc[i], -inc[j], -inc[k]])

            # Link var_end[u] <=> deg == 1
            # If var_end[u] is true -> deg <= 1 (cannot take >= 2 edges)
            for i in range(len(inc)):
                for j in range(i + 1, len(inc)):
                    C.append([-var_end[u], -inc[i], -inc[j]])

            # If var_end[u] is false -> deg >= 2
            if len(inc) == 1:
                # Vertex with internal degree 1 MUST be an endpoint
                C.append([var_end[u]])
            else:
                for i in range(len(inc)):
                    clause = [var_end[u]] + [inc[j] for j in range(len(inc)) if j != i]
                    C.append(clause)

        # 2. Total endpoints = 2K
        target_ends = 2 * K
        ends_list = [var_end[u] for u in verts]

        if target_ends > len(ends_list):
            return False, list(m_demands.keys())

        s_vars = {}
        for i in range(len(ends_list)):
            for j in range(1, target_ends + 1):
                nv += 1
                s_vars[(i, j)] = nv

        # Sinz sequential counter for sum(ends_list) == target_ends
        # i = 0
        x0 = ends_list[0]
        C.append([-x0, s_vars[(0, 1)]])
        for j in range(2, target_ends + 1):
            # s[0, j] can never be true
            C.append([-s_vars[(0, j)]])

        for i in range(1, len(ends_list)):
            xi = ends_list[i]
            # s[i, 1] <= s[i-1, 1] or xi
            C.append([-s_vars[(i - 1, 1)], s_vars[(i, 1)]])
            C.append([-xi, s_vars[(i, 1)]])

            for j in range(2, target_ends + 1):
                C.append([-s_vars[(i - 1, j)], s_vars[(i, j)]])
                C.append([-xi, -s_vars[(i - 1, j - 1)], s_vars[(i, j)]])

            # At most target_ends: xi and s[i-1, target_ends] cannot both be true
            C.append([-xi, -s_vars[(i - 1, target_ends)]])

        # Exactly target_ends: at the end, s[n-1, target_ends] must be true
        C.append([s_vars[(len(ends_list) - 1, target_ends)]])

        # 3. M-Hub demand selector assumptions
        assumption_lits = []
        assumption_map = {}
        for h, req in m_demands.items():
            if req > 0:
                ports = self.decomp.strip_hub_ports.get((si, h), [])
                port_ends = [var_end[p] for p in sorted(set(ports)) if p in var_end]

                nv += 1
                asm_lit = nv
                assumption_lits.append(asm_lit)
                assumption_map[asm_lit] = (h, req)

                if len(port_ends) < req:
                    C.append([-asm_lit])
                elif req == 1:
                    # asm_lit => OR_{p in ports} var_end[p]
                    C.append([-asm_lit] + port_ends)
                elif req == 2:
                    # asm_lit => at least 2
                    C.append([-asm_lit] + port_ends)
                    for pi_idx in range(len(port_ends)):
                        p_var = port_ends[pi_idx]
                        other_vars = [port_ends[j] for j in range(len(port_ends)) if j != pi_idx]
                        C.append([-asm_lit, -p_var] + other_vars)
                else:
                    limit = len(port_ends) - req
                    if limit < 0:
                        C.append([-asm_lit])
                    elif limit == 0:
                        for p_var in port_ends:
                            C.append([-asm_lit, p_var])
                    else:
                        c_vars = {}
                        for i in range(len(port_ends)):
                            for j in range(1, limit + 2):
                                nv += 1
                                c_vars[(i, j)] = nv
                        C.append([-asm_lit, port_ends[0], c_vars[(0, 1)]])
                        for j in range(2, limit + 2):
                            C.append([-c_vars[(0, j)]])
                        for i in range(1, len(port_ends)):
                            pi_var = port_ends[i]
                            C.append([-asm_lit, -c_vars[(i - 1, 1)], c_vars[(i, 1)]])
                            C.append([-asm_lit, pi_var, c_vars[(i, 1)]])
                            for j in range(2, limit + 2):
                                C.append([-asm_lit, -c_vars[(i - 1, j)], c_vars[(i, j)]])
                                C.append([-asm_lit, pi_var, -c_vars[(i - 1, j - 1)], c_vars[(i, j)]])
                            C.append([-asm_lit, pi_var, -c_vars[(i - 1, limit)]])

        # Solve with Cadical195 and assumption core extraction
        with Cadical195(bootstrap_with=C) as solver:
            # Internal CEGAR to prevent closed subtours
            for sub_it in range(50):
                sat = solver.solve(assumptions=assumption_lits)
                if not sat:
                    core = solver.get_core() or []
                    failed_hubs = [assumption_map[l][0] for l in core if l in assumption_map]
                    return False, failed_hubs if failed_hubs else list(m_demands.keys())

                model = solver.get_model()
                m_bool = {abs(x): x > 0 for x in model}

                # Extract paths and cycles
                adj_cov = collections.defaultdict(list)
                for (u, v), vi in var_e.items():
                    if u < v and m_bool.get(vi):
                        adj_cov[u].append(v)
                        adj_cov[v].append(u)

                visited = set()
                paths = []
                cycles = []

                # First extract paths starting from degree-1 endpoints
                endpoints = [u for u in verts if len(adj_cov[u]) == 1]
                for ep in endpoints:
                    if ep not in visited:
                        path = [ep]
                        visited.add(ep)
                        curr = ep
                        prev = None
                        while True:
                            nxts = [w for w in adj_cov[curr] if w != prev]
                            if not nxts:
                                break
                            nxt = nxts[0]
                            path.append(nxt)
                            visited.add(nxt)
                            prev, curr = curr, nxt
                            if len(adj_cov[curr]) == 1:
                                break
                        paths.append(path)

                # Check remaining for cycles
                for u in verts:
                    if u not in visited and len(adj_cov[u]) == 2:
                        cyc = [u]
                        visited.add(u)
                        curr = u
                        prev = None
                        while True:
                            nxts = [w for w in adj_cov[curr] if w != prev]
                            if not nxts or nxts[0] == u:
                                break
                            nxt = nxts[0]
                            cyc.append(nxt)
                            visited.add(nxt)
                            prev, curr = curr, nxt
                        cycles.append(cyc)

                if not cycles and len(paths) == K:
                    return True, paths

                # Add cut/subtour exclusion clauses for internal cycles
                for cyc in cycles:
                    cyc_edges = [var_e[(cyc[i], cyc[(i + 1) % len(cyc)])] for i in range(len(cyc))]
                    solver.add_clause([-e for e in cyc_edges])

        return False, list(m_demands.keys())


def solve_strip_pinpointed(
    strip_idx: int,
    strip_verts: List[int],
    m_demands: Dict[int, int],
    G: Dict[int, Set[int]],
    s_hub: int,
    b_hub: int,
    K: int = 4
) -> Tuple[bool, Any]:
    """Helper functional interface for pinpointed strip solver."""
    class MinimalDecomp:
        def __init__(self):
            self.strips = {strip_idx: strip_verts}
            strip_hub_ports = collections.defaultdict(list)
            for u in strip_verts:
                for nbr in G[u]:
                    if nbr not in strip_verts:
                        strip_hub_ports[(strip_idx, nbr)].append(u)
            self.strip_hub_ports = dict(strip_hub_ports)

    decomp = MinimalDecomp()
    decomp.strips = [strip_verts] if strip_idx == 0 else {strip_idx: strip_verts}
    solver = PinpointedStripSolver(G, decomp)
    return solver.solve_strip(strip_idx, m_demands, s_hub, b_hub, K=K)
