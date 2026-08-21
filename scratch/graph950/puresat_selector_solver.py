#!/usr/bin/env python3
"""100% Pure SAT HCP Solver with Cut-CEGAR for graph950.col.
Zero tour injection / zero external hints.
Pipeline:
  1. Load graph950.col (n=6620, m=28718).
  2. Formulate 2-factor SAT CNF with sound Sinz exact-2 degree counters on all 6620 vertices.
  3. Run Cut-CEGAR subtour elimination loop using CaDiCaL.
  4. Write verified Hamiltonian Cycle tour to /tmp/opencode/found_tour_puresat.hcp.
  5. Run independent certification.
"""
import collections
import os
import subprocess
import sys
import time

def solve_cnf_file(cnf_path, tl=120.0):
    try:
        r = subprocess.run(
            ['/usr/local/bin/cadical', '--quiet', cnf_path],
            capture_output=True,
            text=True,
            timeout=tl
        )
    except subprocess.TimeoutExpired:
        return None, {}
    out = r.stdout
    if 's UNSATISFIABLE' in out:
        return False, {}
    if 's SATISFIABLE' not in out:
        return None, {}
    m = {}
    for line in out.splitlines():
        if line.startswith('v '):
            for x in line.split()[1:]:
                xi = int(x)
                m[abs(xi)] = xi > 0
    return True, m

def main():
    t_start = time.time()
    graph_path = '/home/ubuntu/HCP/FHCPCS-col/graph950.col'
    out_dir = '/tmp/opencode'
    out_file = os.path.join(out_dir, 'found_tour_puresat.hcp')
    cnf_file = '/tmp/graph950_puresat.cnf'
    
    print('=' * 65)
    print('  100% PURE SAT CUT-CEGAR SOLVER FOR graph950')
    print('  (Zero tour injection / Zero external hints)')
    print('=' * 65)
    
    # 1. Load graph
    print('\n[1/4] Loading graph950.col...')
    G = collections.defaultdict(set)
    for l in open(graph_path):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2])
            G[u].add(v)
            G[v].add(u)
    
    verts = sorted(G.keys())
    n_verts = len(verts)
    
    edges = []
    var_e = {}
    for u in verts:
        for v in G[u]:
            if u < v:
                vi = len(edges) + 1
                edges.append((u, v))
                var_e[(u, v)] = vi
                var_e[(v, u)] = vi
    
    print(f'  Vertices: {n_verts}, Edges: {len(edges)}')
    
    # 2. Build 2-factor CNF with sound 2-way Sinz exact-2 counters
    print('\n[2/4] Building 2-Factor SAT CNF...')
    nv = [len(edges)]
    C = []
    
    for u in verts:
        inc = [var_e[(u, w)] for w in sorted(G[u])]
        n = len(inc)
        s = [[nv[0] + 1 + i*3 + j for j in range(3)] for i in range(n)]
        nv[0] += n * 3
        for i in range(n):
            x = inc[i]
            si_ = s[i]
            if i == 0:
                C.append([-x, si_[0]])
                C.append([-si_[1]])
                C.append([-si_[2]])
                C.append([-si_[0], x])
            else:
                prev = s[i-1]
                C.append([-prev[0], si_[0]])
                C.append([-x, si_[0]])
                C.append([-prev[1], si_[1]])
                C.append([-x, -prev[0], si_[1]])
                C.append([-prev[2], si_[2]])
                C.append([-x, -prev[1], si_[2]])
                C.append([-si_[0], prev[0], x])
                C.append([-si_[1], prev[1], prev[0]])
                C.append([-si_[1], prev[1], x])
                C.append([-si_[2], prev[2], prev[1]])
                C.append([-si_[2], prev[2], x])
        C.append([s[-1][1]])
        C.append([-s[-1][2]])
    
    print(f'  Base 2-Factor CNF: {len(C)} clauses, {nv[0]} variables (built in {time.time()-t_start:.2f}s)')
    
    # 3. Cut-CEGAR Subtour Elimination Loop
    print('\n[3/4] Running Cut-CEGAR Subtour Elimination Loop...')
    cut_clauses = []
    solved = False
    final_cycle = None
    t_cegar = time.time()
    
    for it in range(1, 100):
        if time.time() - t_start > 1750:
            print('  GLOBAL 1800s TIMEOUT REACHED', flush=True)
            break
        
        # Write CNF file
        total_clauses = len(C) + len(cut_clauses)
        with open(cnf_file, 'w') as f:
            f.write(f'p cnf {nv[0]} {total_clauses}\n')
            for c in C:
                f.write(' '.join(map(str, c + [0])) + '\n')
            for c in cut_clauses:
                f.write(' '.join(map(str, c + [0])) + '\n')
        
        t_it = time.time()
        sat, m = solve_cnf_file(cnf_file, tl=180.0)
        
        if sat is None:
            print(f'  Iter {it:2d}: CaDiCaL Timeout (>180s)', flush=True)
            break
        if sat is False:
            print(f'  Iter {it:2d}: CaDiCaL returned UNSAT', flush=True)
            break
        
        # Extract 2-factor active edges
        adj = collections.defaultdict(list)
        for (u, v), vi in var_e.items():
            if u < v and m.get(vi):
                adj[u].append(v)
                adj[v].append(u)
        
        # Extract connected cycles
        visited = set()
        cycles = []
        for v0 in verts:
            if v0 not in visited:
                cyc = [v0]
                visited.add(v0)
                cur = v0
                prev = None
                while True:
                    nxts = [w for w in adj.get(cur, []) if w != prev]
                    if not nxts:
                        break
                    nxt = nxts[0]
                    if nxt == v0:
                        break
                    cyc.append(nxt)
                    visited.add(nxt)
                    prev, cur = cur, nxt
                cycles.append(cyc)
        
        print(f'  Iter {it:2d}: {len(cycles):3d} cycles | Max cycle: {max(len(c) for c in cycles):4d}/{n_verts} | Cut clauses: {len(cut_clauses):4d} ({time.time()-t_it:.2f}s)', flush=True)
        
        if len(cycles) == 1 and len(cycles[0]) == n_verts:
            print(f'\n  *** PURE SAT CONVERGED: Single Hamiltonian Cycle on all {n_verts} Vertices! ***')
            print(f'  Total CEGAR Time: {time.time()-t_cegar:.2f}s across {it} iterations')
            solved = True
            final_cycle = cycles[0]
            break
        
        # Add Cut-Crossing Clauses for all subcycles
        # For each cycle C_i, cut clause: OR_{(u, v) in E, u in C_i, v not in C_i} var_e(u, v)
        for cyc in cycles:
            C_set = set(cyc)
            if len(C_set) < n_verts:
                cut_edges = []
                for u in cyc:
                    for w in G[u]:
                        if w not in C_set:
                            cut_edges.append(var_e[(u, w)])
                if cut_edges:
                    cut_clauses.append(cut_edges)
    
    # 4. Certification & Output
    if solved and final_cycle is not None:
        print('\n[4/4] Verifying and Writing Tour...')
        full = final_cycle + [final_cycle[0]]
        
        # Independent validation
        bad_edges = []
        for i in range(len(full) - 1):
            if full[i+1] not in G[full[i]]:
                bad_edges.append((i, full[i], full[i+1]))
        
        ok_len = (len(full) == n_verts + 1 and len(set(final_cycle)) == n_verts)
        ok_edges = (len(bad_edges) == 0)
        
        print(f'  Cycle Length: {len(full)-1}/{n_verts}')
        print(f'  Distinct Vertices: {len(set(final_cycle))}/{n_verts}')
        print(f'  Bad Edges in G: {len(bad_edges)}')
        
        if ok_len and ok_edges:
            os.makedirs(out_dir, exist_ok=True)
            with open(out_file, 'w') as f:
                f.write(f'NAME: graph950_puresat\nTYPE: TOUR\nDIMENSION: {n_verts}\nTOUR_SECTION\n')
                f.write('\n'.join(map(str, final_cycle)) + '\n-1\nEOF\n')
            print(f'\n  *** INDEPENDENT CERTIFICATION SUCCESSFUL: 100% VALID HAMILTONIAN CYCLE ***')
            print(f'  Tour saved to: {out_file}')
            print(f'  Total Execution Time: {time.time()-t_start:.2f}s')
            return 0
        else:
            print('  *** CERTIFICATION FAILED ***')
            return 1
    else:
        print('  Failed to find Hamiltonian cycle.')
        return 1

if __name__ == '__main__':
    sys.exit(main())
