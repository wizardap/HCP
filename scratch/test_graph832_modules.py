import collections, time, os, sys
sys.path.insert(0, ".")
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.bipartite_module_detector import detect_variable_modules
from scratch.engine.dual_path_extractor import extract_module_dual_paths
from scratch.engine.assembler import export_hcp_tour
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def solve_graph832_modular():
    col_path = "FHCPCS-col/graph832.col"
    t0 = time.time()
    print(f"[*] Loading {col_path}...")
    G = load_dimacs(col_path)
    nv = len(G)
    
    # 1. Detect modules
    modules = detect_variable_modules(G, set(G.keys()), [])
    print(f"[*] Detected {len(modules)} modules in graph832.")
    
    # 2. Contract degree-2 vertices in G
    rem = set(G.keys())
    adj = {u: set(G[u]) for u in G}
    contracted_edges = set()
    edge_chains = {}
    while True:
        d2 = [u for u in rem if len(adj[u]) == 2]
        if not d2: break
        v = d2[0]
        u, w = list(adj[v])
        adj[u].remove(v); adj[w].remove(v); del adj[v]; rem.remove(v)
        adj[u].add(w); adj[w].add(u)
        e_new = tuple(sorted([u, w]))
        contracted_edges.add(e_new)
        
    edges = sorted(list({tuple(sorted([u, v])) for u in rem for v in adj[u]}))
    edge_to_var = {e: i + 1 for i, e in enumerate(edges)}
    
    solver = Cadical195()
    top = len(edges) + 1
    inc_edges = collections.defaultdict(list)
    for e in edges:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])
        
    for u in rem:
        cl = CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for c in cl:
            solver.add_clause(c)
            for lit in c: top = max(top, abs(lit) + 1)
            
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])
        
    # 3. Greedy module edge fixing
    fixed_lits = []
    fixed_modules = 0
    for idx, m in enumerate(modules):
        t_path, f_path = extract_module_dual_paths(G, m)
        ves = {tuple(sorted(e)) for e in m.get('virtual_edges', ())}
        has_t = ves.issubset(t_path) if t_path else False
        if has_t:
            lits = [edge_to_var[e] for e in t_path if e in edge_to_var]
            if solver.solve(assumptions=fixed_lits + lits):
                fixed_lits.extend(lits)
                fixed_modules += 1
                for lit in lits:
                    solver.add_clause([lit])
                print(f"    Fixed Module {idx+1}: +{len(lits)} edges (total {len(fixed_lits)}).")
                
    print(f"[*] Successfully fixed {fixed_modules} modules ({len(fixed_lits)} edges) at Level 0.")
    
    # 4. CEGAR loop with Giant Preservation and boundary cuts
    round_num = 0
    seen_cuts = set()
    while True:
        round_num += 1
        t_r = time.time()
        res = solver.solve()
        t_solve = time.time() - t_r
        if not res:
            print("UNSAT!")
            return False
            
        model = set(solver.get_model())
        active = [e for e in edges if edge_to_var[e] in model]
        act_adj = {u: [] for u in rem}
        for u, v in active:
            act_adj[u].append(v); act_adj[v].append(u)
            
        visited = set()
        cycles = []
        for u in rem:
            if u not in visited:
                c = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); c.append(curr)
                    nbrs = act_adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(c)
                
        cycles.sort(key=len, reverse=True)
        top5 = [len(c) for c in cycles[:5]]
        print(f"Round {round_num:2d} ({t_solve*1000:6.1f}ms): {len(cycles):3d} cycles, Giant: {len(cycles[0])}/{len(rem)} ({100*len(cycles[0])/len(rem):.1f}%), top5: {top5}", flush=True)
        
        if len(cycles) == 1:
            print(f"*** FOUND HAMILTONIAN CYCLE in {round_num} rounds, {time.time()-t0:.2f}s! ***", flush=True)
            # Reconstruct full tour
            return cycles[0]
            
        # Cuts:
        # Negative cut only for small cycles (<= len(rem) // 2) or when len(cycles) == 2
        for c in cycles:
            if len(c) <= len(rem) // 2 or len(cycles) == 2:
                neg_clause = [-edge_to_var[tuple(sorted([c[i], c[(i+1)%len(c)]]))] for i in range(len(c))]
                t_neg = tuple(sorted(neg_clause))
                if t_neg not in seen_cuts:
                    seen_cuts.add(t_neg)
                    solver.add_clause(neg_clause)
                    
            # Boundary cocycle cut for ALL cycles (forces connectivity)
            c_set = set(c)
            cut_e = [tuple(sorted([u, v])) for u in c for v in adj[u] if v not in c_set]
            c_clause = [edge_to_var[e] for e in cut_e]
            t_cut = tuple(sorted(c_clause))
            if t_cut not in seen_cuts:
                seen_cuts.add(t_cut)
                solver.add_clause(c_clause)

if __name__ == "__main__":
    solve_graph832_modular()
