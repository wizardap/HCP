import collections
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195

def detect_variable_modules(
    G: Dict[int, Set[int]],
    comp0_nodes: Set[int],
    virtual_edges: List[Tuple[int, int]]
) -> List[Dict]:
    """
    Clusters variable modules (44-vertex or 88-vertex gadgets) in Comp 0.
    Identifies interface ports and internal nodes for modular state equivalence encoding.
    """
    if not comp0_nodes:
        return []

    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports_corridor = set()
    for u, v in virt_set:
        ports_corridor.add(u)
        ports_corridor.add(v)

    c0_set = set(comp0_nodes)
    adj_c0 = {u: set(v for v in G.get(u, ()) if v in c0_set) for u in comp0_nodes}
    for u, v in virt_set:
        if u in adj_c0 and v in adj_c0:
            adj_c0[u].add(v)
            adj_c0[v].add(u)

    # 1. Identify degree-2 vertices in Comp 0
    deg2 = [u for u in comp0_nodes if u not in ports_corridor and len(adj_c0[u]) == 2]
    v_partner = {}
    for v in deg2:
        nbrs = list(adj_c0[v])
        if len(nbrs) == 2:
            u, w = nbrs[0], nbrs[1]
            v_partner[u] = w
            v_partner[w] = u

    contracted_nodes = set(v_partner.keys())
    if not contracted_nodes:
        return []

    # Subgraph on contracted endpoints
    adj_contracted = {u: set(v for v in adj_c0[u] if v in contracted_nodes) for u in contracted_nodes}

    # 2. Cluster elementary gadgets (typically 22 vertices each)
    d2_in_c = [u for u in contracted_nodes if len(adj_contracted[u]) == 2]
    gadget_adj = collections.defaultdict(set)
    for u in d2_in_c:
        for v in adj_contracted[u]:
            gadget_adj[u].add(v)
            gadget_adj[v].add(u)
    for u, w in v_partner.items():
        gadget_adj[u].add(w)
        gadget_adj[w].add(u)

    visited = set()
    elementary_gadgets = []
    for u in sorted(contracted_nodes):
        if u not in visited:
            comp = []
            q = [u]
            visited.add(u)
            for curr in q:
                comp.append(curr)
                for nxt in gadget_adj[curr]:
                    if nxt not in visited:
                        visited.add(nxt)
                        q.append(nxt)
            if len(comp) >= 10:
                elementary_gadgets.append(sorted(comp))

    if not elementary_gadgets:
        return []

    num_gadgets = len(elementary_gadgets)
    node_to_eg = {}
    for gid, g_nodes in enumerate(elementary_gadgets):
        for u in g_nodes:
            node_to_eg[u] = gid

    # Precompute internal Hamiltonian path transitions within each elementary gadget
    def get_hp_pairs(g_nodes):
        vert_set = set(g_nodes)
        real_nbrs = collections.defaultdict(list)
        for u in g_nodes:
            for v in adj_c0[u]:
                if v in vert_set and v_partner.get(u) != v:
                    real_nbrs[u].append(v)
        paths = []
        def dfs(curr, path, vis):
            if len(path) == len(g_nodes):
                paths.append((path[0], path[-1]))
                return
            if len(path) % 2 == 1:
                nxt = v_partner.get(curr)
                if nxt and nxt not in vis:
                    vis.add(nxt)
                    path.append(nxt)
                    dfs(nxt, path, vis)
                    path.pop()
                    vis.remove(nxt)
            else:
                for nxt in real_nbrs[curr]:
                    if nxt not in vis:
                        vis.add(nxt)
                        path.append(nxt)
                        dfs(nxt, path, vis)
                        path.pop()
                        vis.remove(nxt)
        for start in g_nodes:
            dfs(start, [start], {start})
        transitions = collections.defaultdict(set)
        for a, b in paths:
            transitions[a].add(b)
            transitions[b].add(a)
        return transitions

    gadget_hp = [get_hp_pairs(g) for g in elementary_gadgets]

    # Discover valid 4-gadget clusters that admit end-to-end spanning paths
    valid_4_clusters = {}
    for g0 in range(num_gadgets):
        for p0_in, p0_outs in gadget_hp[g0].items():
            for p0_out in p0_outs:
                for p1_in in adj_c0[p0_out]:
                    if p1_in not in node_to_eg:
                        continue
                    g1 = node_to_eg[p1_in]
                    if g1 == g0 or v_partner.get(p0_out) == p1_in:
                        continue
                    if p1_in in gadget_hp[g1]:
                        for p1_out in gadget_hp[g1][p1_in]:
                            for p2_in in adj_c0[p1_out]:
                                if p2_in not in node_to_eg:
                                    continue
                                g2 = node_to_eg[p2_in]
                                if g2 == g0 or g2 == g1 or v_partner.get(p1_out) == p2_in:
                                    continue
                                if p2_in in gadget_hp[g2]:
                                    for p2_out in gadget_hp[g2][p2_in]:
                                        for p3_in in adj_c0[p2_out]:
                                            if p3_in not in node_to_eg:
                                                continue
                                            g3 = node_to_eg[p3_in]
                                            if g3 == g0 or g3 == g1 or g3 == g2 or v_partner.get(p2_out) == p3_in:
                                                continue
                                            if p3_in in gadget_hp[g3]:
                                                for p3_out in gadget_hp[g3][p3_in]:
                                                    cl = tuple(sorted([g0, g1, g2, g3]))
                                                    if cl not in valid_4_clusters:
                                                        valid_4_clusters[cl] = (p0_in, p3_out)

    cl_list = sorted(list(valid_4_clusters.keys()))
    chosen_clusters = []
    if num_gadgets % 4 == 0 and len(cl_list) > 0:
        with Cadical195() as solver:
            for gid in range(num_gadgets):
                inc = [idx + 1 for idx, cl in enumerate(cl_list) if gid in cl]
                solver.add_clause(inc)
                for i in range(len(inc)):
                    for j in range(i + 1, len(inc)):
                        solver.add_clause([-inc[i], -inc[j]])
            if solver.solve():
                model = set(solver.get_model())
                chosen_clusters = [cl_list[idx] for idx in range(len(cl_list)) if (idx + 1) in model]

    if not chosen_clusters:
        # Fallback greedy packing of disjoint 4-clusters
        used = set()
        for cl in cl_list:
            if not any(g in used for g in cl):
                used.update(cl)
                chosen_clusters.append(cl)

    # If still no 4-clusters (e.g. non-modular or small), fall back to individual elementary gadgets
    if not chosen_clusters:
        chosen_clusters = [(gid,) for gid in range(num_gadgets)]

    modules = []
    for mid, cl in enumerate(chosen_clusters):
        m_nodes = []
        for gid in cl:
            m_nodes.extend(elementary_gadgets[gid])
        m_set = set(m_nodes)
        ext_ports = []
        for u in m_nodes:
            ext_nbrs = [v for v in adj_contracted[u] if v not in m_set]
            if ext_nbrs:
                ext_ports.append(u)

        m_ves = set()
        for u in m_nodes:
            if u in v_partner and v_partner[u] in m_set and u < v_partner[u]:
                m_ves.add((u, v_partner[u]))

        pin, pout = valid_4_clusters.get(cl, (None, None))
        if pin in ext_ports and pout in ext_ports and pin != pout:
            ports = tuple(sorted([pin, pout]))
        elif len(ext_ports) >= 2:
            ports = tuple(sorted(ext_ports[:2]))
        else:
            ports = tuple(sorted(ext_ports))

        int_nodes = m_set - set(ports)

        modules.append({
            'id': mid,
            'nodes': m_set,
            'virtual_edges': m_ves,
            'ports': ports,
            'internal_nodes': int_nodes,
        })

    return modules
