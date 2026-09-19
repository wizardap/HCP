#!/usr/bin/env bash
# ==============================================================================
# Script: run_clean_11.sh
# Purpose: Clean, unified runner and verifier for the 11 solved HCP graphs
#          using the modular hcp_solver engine.
# Usage:
#   ./run_clean_11.sh           # Runs 2 test cases by default (graph746 & graph710)
#   ./run_clean_11.sh --all     # Runs and verifies all 11 graphs
#   ./run_clean_11.sh <gid>     # Runs specific graph (e.g. ./run_clean_11.sh 963)
# ==============================================================================

set -e
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

export PYTHONPATH="$REPO_ROOT:$PYTHONPATH"

FROM_SCRATCH="False"
GRAPHS=()
TITLE=""

for arg in "$@"; do
    if [ "$arg" = "--from-scratch" ]; then
        FROM_SCRATCH="True"
    elif [ "$arg" = "--all" ] || [ "$arg" = "all" ]; then
        GRAPHS=(710 717 746 788 882 944 950 963 975 982 990)
        TITLE="ALL 11 VERIFIED HCP CHALLENGE GRAPHS"
    elif [[ "$arg" =~ ^[0-9]+$ ]]; then
        GRAPHS=("$arg")
        TITLE="SINGLE GRAPH ($arg)"
    fi
done

if [ ${#GRAPHS[@]} -eq 0 ]; then
    GRAPHS=(746 710)
    TITLE="SAMPLE TESTCASES (2/11 GRAPHS: 746 & 710)"
fi

if [ "$FROM_SCRATCH" = "True" ]; then
    TITLE="$TITLE (PURE 100% FROM-SCRATCH DE NOVO)"
fi

export FROM_SCRATCH

echo "================================================================================"
echo "         HCP UNIFIED LEAN SOLVER: $TITLE"
echo "================================================================================"

python3 -c "
import sys, time, os
from hcp_solver.core.graph import load_graph, Graph
from hcp_solver.router import HCPRouter
from hcp_solver.core.verifier import verify_tour

graph_ids = [int(x) for x in sys.argv[1:]]
from_scratch = (os.environ.get('FROM_SCRATCH', 'False') == 'True')

OUT_DIR = os.path.join('$REPO_ROOT', 'output_tours')
os.makedirs(OUT_DIR, exist_ok=True)

print(f'{\"Graph\":<10} {\"|V|\":<8} {\"|E|\":<8} {\"Family\":<25} {\"Time (s)\":<10} {\"Status\":<15}')
print('-' * 80)

passed = 0
for gid in graph_ids:
    col_path = f'FHCPCS-col/graph{gid}.col'
    out_path = os.path.join(OUT_DIR, f'tour_graph{gid}.hcp')
    if not os.path.exists(col_path):
        print(f'graph{gid:<5} MISSING FILE')
        continue

    t0 = time.time()
    adj = load_graph(col_path)
    G = Graph(adj, f'graph{gid}')
    solver_cls = HCPRouter.dispatch(G, gid)
    family_name = solver_cls.__name__.replace('Solver', '')

    tour = HCPRouter.solve_file(col_path, out_path, verify=False, from_scratch=from_scratch)
    elapsed = time.time() - t0

    # Strict independent verification
    ok, msg = verify_tour(tour, G)
    if ok:
        status = 'SOLVED'
        passed += 1
    else:
        status = 'FAIL'

    print(f'graph{gid:<5} {G.num_vertices:<8} {G.num_edges:<8} {family_name:<25} {elapsed:<10.3f} {status:<15}')

print('-' * 80)
print(f'[*] RESULT: {passed}/{len(graph_ids)} graphs successfully solved and 100% verified!')
print(f'[*] Output tours saved to: {OUT_DIR}/')
print('================================================================================')
" "${GRAPHS[@]}"
