#!/usr/bin/env bash
# ==============================================================================
# Script: run_11_solved.sh
# Purpose: Dedicated fast runner and verifier for the 11 SOLVED Challenge Graphs:
#          710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990.
# ==============================================================================

set -e
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

echo "================================================================================"
echo "    VERIFICATION & REPRODUCTION SUITE FOR THE 11 SOLVED HCP GRAPHS"
echo "================================================================================"

# Verify environment
if ! python3 -c "import sys; sys.exit(0)" 2>/dev/null; then
    echo "[-] Error: python3 is required."
    exit 1
fi

python3 -c "
import os, sys

REPO_ROOT = '$REPO_ROOT'

SOLVED_GRAPHS = {
    710: ('scratch/graph710/found_tour_graph710.hcp', 'Corridor Bridge Decomposition + Degree-2 Contraction'),
    717: ('scratch/graph717/found_tour_graph717.hcp', '6-Module Parallel SAT + Degree-2 Chain Splicing'),
    746: ('scratch/graph746/found_tour_graph746.hcp', '5-Cluster Macro-Ring Hierarchical Bipartite Solver'),
    788: ('scratch/graph788/found_tour_graph788.hcp', 'Block Contraction + Deterministic Splicing & DP Bitmask'),
    882: ('scratch/graph882/found_tour_graph882.hcp', 'Two-Corridor micro-SAT + Comp0 Contraction & CEGAR'),
    944: ('scratch/engine/found_tour_graph944.hcp',  'Multi-Corridor micro-SAT + 14v Local SAT Splicer'),
    950: ('scratch/graph950/found_tour_puresat.hcp',  'Two-Half 10-Group Bipartite Cluster Decomposition'),
    963: ('scratch/graph963/found_tour_graph963.hcp', 'Two-Half 10-Group Bipartite Cluster Decomposition'),
    975: ('scratch/graph975/found_tour_graph975.hcp', 'Two-Half 10-Group Bipartite Cluster Decomposition'),
    982: ('scratch/graph982/found_tour_graph982.hcp', 'Two-Half 10-Group Bipartite Cluster Decomposition'),
    990: ('scratch/graph990/found_tour_graph990.hcp', 'Two-Half 10-Group Bipartite Cluster Decomposition'),
}

def parse_col(path):
    adj = {}
    with open(path) as f:
        for line in f:
            if line.startswith('e '):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                if u not in adj: adj[u] = set()
                if v not in adj: adj[v] = set()
                adj[u].add(v); adj[v].add(u)
    return len(adj), sum(len(x) for x in adj.values()) // 2, adj

def parse_tour(path):
    tour = []
    in_tour = False
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('c') or line.startswith('NAME') or line.startswith('TYPE') or line.startswith('DIMENSION'):
                continue
            if line == 'TOUR_SECTION':
                in_tour = True
                continue
            if line in ('-1', 'EOF'):
                break
            if in_tour:
                try:
                    tour.append(int(line))
                except ValueError:
                    pass
    return tour

print(f'{\"Graph\":<10} {\"|V|\":<7} {\"|E|\":<8} {\"Status\":<16} {\"Solving Method\":<45}')
print('-' * 90)

pass_count = 0
for gid in sorted(SOLVED_GRAPHS.keys()):
    t_rel, method = SOLVED_GRAPHS[gid]
    col_rel = f'FHCPCS-col/graph{gid}.col'
    col_path = os.path.join(REPO_ROOT, col_rel)
    tour_path = os.path.join(REPO_ROOT, t_rel)

    if not os.path.exists(col_path):
        print(f'graph{gid:<5} MISSING RAW GRAPH')
        continue
    if not os.path.exists(tour_path):
        print(f'graph{gid:<5} MISSING TOUR FILE')
        continue

    nv, ne, adj = parse_col(col_path)
    tour = parse_tour(tour_path)

    # Rigorous Verification:
    # 1. Exact vertex count
    assert len(tour) == nv, f'Tour length {len(tour)} != {nv}'
    # 2. Zero duplicates (Hamiltonian)
    assert len(set(tour)) == nv, f'Duplicate vertices detected!'
    # 3. Exactly matches graph vertex set
    assert set(tour) == set(adj.keys()), f'Tour vertex set does not match graph vertices!'
    # 4. 100% valid DIMACS edges
    for i in range(nv):
        u = tour[i]
        v = tour[(i + 1) % nv]
        assert v in adj[u], f'Phantom edge ({u}, {v}) not in raw DIMACS graph!'

    pass_count += 1
    print(f'graph{gid:<5} {nv:<7} {ne:<8} {\"PASS (SOUND)\":<16} {method:<45}')

print('-' * 90)
print(f'[*] RESULT: {pass_count}/{len(SOLVED_GRAPHS)} Graphs 100% MATHEMATICALLY VERIFIED & SOUND!')
print('================================================================================')
"
