#!/usr/bin/env bash
# ==============================================================================
# Script: run_solver.sh
# Purpose: Universal, 100% Router-Free HCP Solver Runner
# Usage:
#   ./run_solver.sh FHCPCS-col/graph1.col    # Solve a single graph file
#   ./run_solver.sh 1 50                    # Batch solve graphs in range [1, 50]
#   ./run_solver.sh --challenge             # Batch solve all 11 challenge graphs
# ==============================================================================

set -e
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"
export PYTHONPATH="$REPO_ROOT:$PYTHONPATH"

OUT_DIR="$REPO_ROOT/output_tours"
mkdir -p "$OUT_DIR"

if [ "$1" = "--challenge" ]; then
    GRAPHS=(710 717 746 788 882 944 950 963 975 982 990)
    echo "================================================================================"
    echo "         RUNNING 11 HCP CHALLENGE BENCHMARK GRAPHS"
    echo "================================================================================"
    python3 -c "
import sys, time, os
from hcp_solver import solve_general_hcp, load_graph, Graph, verify_tour

graph_ids = [710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990]
print(f'{\"Graph\":<10} {\"|V|\":<8} {\"|E|\":<8} {\"Time (s)\":<10} {\"Status\":<15}')
print('-' * 60)
passed = 0
for gid in graph_ids:
    col = f'FHCPCS-col/graph{gid}.col'
    if not os.path.exists(col):
        print(f'graph{gid:<5} MISSING FILE')
        continue
    t0 = time.time()
    adj = load_graph(col)
    G = Graph(adj, f'graph{gid}')
    tour = solve_general_hcp(col, verbose=False)
    dt = time.time() - t0
    ok, msg = verify_tour(tour, G)
    status = '100% SOUND' if ok else 'FAILED'
    if ok: passed += 1
    print(f'graph{gid:<5} {len(tour):<8} {G.num_edges:<8} {dt:<10.3f} {status:<15}')
print('-' * 60)
print(f'[*] RESULT: {passed}/{len(graph_ids)} challenge graphs solved and certified 100% SOUND!')
"
elif [ -f "$1" ]; then
    # Single file mode
    python3 -m hcp_solver "$1" -o "$OUT_DIR/tour_$(basename "$1" .col).hcp"
elif [[ "$1" =~ ^[0-9]+$ ]] && [[ "$2" =~ ^[0-9]+$ ]]; then
    # Range mode
    START="$1"
    END="$2"
    echo "================================================================================"
    echo "         RUNNING BATCH GENERAL GRAPHS [graph$START .. graph$END]"
    echo "================================================================================"
    python3 -c "
import sys, time, os
from hcp_solver import solve_general_hcp, load_graph, Graph, verify_tour

start, end = int(sys.argv[1]), int(sys.argv[2])
passed = 0
total = 0
print(f'{\"Graph\":<10} {\"|V|\":<8} {\"|E|\":<8} {\"Time (s)\":<10} {\"Status\":<15}')
print('-' * 60)
for gid in range(start, end + 1):
    col = f'FHCPCS-col/graph{gid}.col'
    if not os.path.exists(col):
        continue
    total += 1
    t0 = time.time()
    adj = load_graph(col)
    G = Graph(adj, f'graph{gid}')
    try:
        tour = solve_general_hcp(col, timeout_sec=30.0, verbose=False)
        dt = time.time() - t0
        ok, msg = verify_tour(tour, G)
        status = '100% SOUND' if ok else 'FAILED'
        if ok: passed += 1
    except Exception as e:
        dt = time.time() - t0
        status = 'TIMEOUT/FAIL'
    print(f'graph{gid:<5} {G.num_vertices:<8} {G.num_edges:<8} {dt:<10.3f} {status:<15}')
print('-' * 60)
print(f'[*] RESULT: {passed}/{total} graphs solved and certified 100% SOUND!')
" "$START" "$END"
else
    echo "Usage:"
    echo "  $0 <file.col>       # Solve single file"
    echo "  $0 <start> <end>    # Solve range of graphs (e.g. $0 1 50)"
    echo "  $0 --challenge      # Solve all 11 challenge graphs"
    exit 1
fi
