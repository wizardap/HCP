import sys, time, collections
sys.path.insert(0, ".")
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.sat_merger import sat_merge_cycles
from scratch.engine.comp0_solver import try_merge_2opt, try_merge_3opt, try_patch_merge
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def test_greedy_absorb_832():
    col_path = "FHCPCS-col/graph832.col"
    t0 = time.time()
    print(f"[*] Loading {col_path}...")
    G = load_dimacs(col_path)
    nv = len(G)
    
    # Contract degree-2
    adj_c0 = collections.defaultdict(set)
    for u in G:
        adj_c0[u] = set(G[u])
        
    rem = set(G.keys())
    edge_chains = {}
    while True:
        d2 = [u for u in sorted(rem) if len(adj_c0[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v)
        adj_c0[w].remove(v)
        del adj_c0[v]
        rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v])
        c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj_c0[u].add(w)
        adj_c0[w].add(u)
        
    n_c = len(rem)
    contracted_edges = set(edge_chains.keys())
    forbidden_delete = set(contracted_edges)
    print(f"[*] Contracted: {nv} -> {n_c} vertices.")
    
    edges = set()
    for u in sorted(rem):
        for v in sorted(adj_c0[u]):
            if u < v: edges.add((u, v))
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])
        
    static_clauses = []
    top = len(edge_list) + 1
    for u in sorted(rem):
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            static_clauses.append(cl)
            for lit in cl: top = max(top, abs(lit) + 1)
            
    for ce in contracted_edges:
        static_clauses.append([edge_to_var[ce]])
        
    solver = Cadical195(bootstrap_with=static_clauses)
    seen_cuts = set()
    
    it = 0
    while True:
        it += 1
        t_it = time.time()
        if not solver.solve():
            print("UNSAT!")
            return False
            
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)
            
        visited = set()
        cycles = []
        for u in sorted(rem):
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)
                
        cycles.sort(key=len, reverse=True)
        giant_len = len(cycles[0])
        
        # ACTIVE GREEDY ABSORPTION:
        # If giant cycle has >= 40% of vertices, greedily absorb ALL possible small cycles!
        merged = list(cycles)
        absorb_count = 0
        
        # 1. 2-opt pass
        for i in range(len(merged)):
            j = 1
            while j < len(merged):
                res = try_merge_2opt(merged[0], merged[j], adj_c0, forbidden_delete)
                if res is not None:
                    merged[0] = res
                    merged.pop(j)
                    absorb_count += 1
                else:
                    j += 1
                    
        # 2. SAT-HP / patch / 3-opt absorb into Giant for ALL remaining small cycles
        j = 1
        while j < len(merged):
            small = merged[j]
            res = sat_merge_cycles(merged[0], small, adj_c0, forbidden_delete, max_window=20)
            if res is None:
                res = try_patch_merge(merged[0], small, adj_c0, forbidden_delete, max_window=15)
            if res is None and len(small) <= 30:
                res = try_merge_3opt(merged[0], small, adj_c0, forbidden_delete)
            if res is not None:
                merged[0] = res
                merged.pop(j)
                absorb_count += 1
            else:
                j += 1
                
        print(f"Iter {it:2d} ({time.time()-t_it:5.2f}s, total {time.time()-t0:5.1f}s): {len(cycles):3d} raw (Giant {giant_len}) -> {len(merged):2d} after absorb (+{absorb_count} absorbed, Giant {len(merged[0])}/{n_c})", flush=True)
        
        if len(merged) == 1:
            print(f"*** 100% HAMILTONIAN CYCLE FOUND on graph832 in {time.time()-t0:.2f}s! ***")
            full_cycle = merged[0]
            # Uncontract
            uncontracted = []
            for i in range(len(full_cycle)):
                u = full_cycle[i]
                v = full_cycle[(i + 1) % len(full_cycle)]
                e = tuple(sorted([u, v]))
                if e in edge_chains:
                    chain = edge_chains[e]
                    if chain[0] != u: chain = list(reversed(chain))
                    uncontracted.extend(chain[:-1])
                else:
                    uncontracted.append(u)
            print(f"Full uncontracted tour length: {len(uncontracted)} (expected {nv})")
            
            # Verify
            assert len(uncontracted) == nv, f"Length {len(uncontracted)} != {nv}"
            assert len(set(uncontracted)) == nv, "Duplicates!"
            for i in range(len(uncontracted)):
                u = uncontracted[i]
                v = uncontracted[(i + 1) % len(uncontracted)]
                assert v in G[u], f"Missing edge ({u}, {v})!"
            print("100% SOUND AND CERTIFIED HAMILTONIAN CYCLE ON GRAPH832!")
            
            # Write tour
            out_path = "scratch/graph832/found_tour_graph832.hcp"
            os.makedirs(os.path.dirname(out_path), exist_ok=True)
            with open(out_path, "w") as f:
                f.write(f"NAME: graph832\nTYPE: TOUR\nDIMENSION: {nv}\nTOUR_SECTION\n")
                for node in uncontracted:
                    f.write(f"{node}\n")
                f.write("-1\nEOF\n")
            print(f"Tour saved to {out_path}")
            return True
            
        # Cuts:
        for cyc in cycles:
            # Giant preservation: only add negative cuts to cycles <= n_c // 2
            if len(cyc) <= n_c // 2:
                neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
                t_neg = tuple(sorted(neg_c))
                if t_neg not in seen_cuts:
                    seen_cuts.add(t_neg); solver.add_clause(neg_c)
                    
            # Boundary cocycle cuts for all cycles
            c_set = set(cyc)
            cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
            c_cl = [edge_to_var[e] for e in cut_e]
            t_cut = tuple(sorted(c_cl))
            if t_cut not in seen_cuts:
                seen_cuts.add(t_cut); solver.add_clause(c_cl)

if __name__ == "__main__":
    test_greedy_absorb_832()
