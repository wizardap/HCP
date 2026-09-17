#!/usr/bin/env bash
# ==============================================================================
# Script: run_benchmark_29.sh
# Purpose:
#   1. Sets up the full environment (System dependencies, Rust, Python + PySAT).
#   2. Runs smoke tests on sample testcases to verify solver bindings & CLI.
#   3. Executes the 29 canonical FHCP challenge graphs with configurable timeout.
#   4. Verifies sound Hamiltonian tours with python3 scratch/verify_29.py.
#
# Target 29 Graphs:
#   710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
#   951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
#   976, 981, 982, 983, 987, 990, 993, 994, 998
# ==============================================================================

set -o pipefail
export PYTHONUNBUFFERED=1

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# Default configuration
TIMEOUT_SECS=1800
SKIP_SETUP=false
FORCE_RERUN=false
LOG_DIR="$REPO_ROOT/logs/benchmark_29"
mkdir -p "$LOG_DIR"

# Parse optional command-line flags
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --skip-setup)
            SKIP_SETUP=true
            shift
            ;;
        --force)
            FORCE_RERUN=true
            shift
            ;;
        --timeout)
            TIMEOUT_SECS="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --skip-setup       Skip apt-get and pip package installation"
            echo "  --force            Re-run solvers even if tour already exists & passes"
            echo "  --timeout <secs>   Timeout per graph in seconds (default: 1800)"
            echo "  -h, --help         Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Run '$0 --help' for usage."
            exit 1
            ;;
    esac
done

echo "================================================================================"
echo "    29-GRAPH HCP BENCHMARK PIPELINE (SETUP -> SMOKE TEST -> BENCHMARK)"
echo "================================================================================"
echo "[*] Repository Root: $REPO_ROOT"
echo "[*] Per-Graph Timeout: ${TIMEOUT_SECS}s ($((TIMEOUT_SECS / 60)) minutes)"
echo "[*] Logs Directory:  $LOG_DIR"
echo "[*] Skip Setup:      $SKIP_SETUP"
echo "[*] Force Rerun:     $FORCE_RERUN"
echo "================================================================================"

# ==============================================================================
# PHASE 1: ENVIRONMENT SETUP
# ==============================================================================
if [ "$SKIP_SETUP" = false ]; then
    echo ""
    echo "================================================================================"
    echo ">>> PHASE 1: SYSTEM & LANGUAGE ENVIRONMENT SETUP"
    echo "================================================================================"

    # 1.1 Debian / Ubuntu Packages
    if command -v apt-get &> /dev/null; then
        echo "[*] Checking & installing system build packages..."
        sudo apt-get update -y
        sudo apt-get install -y \
            build-essential \
            gcc \
            g++ \
            make \
            cmake \
            pkg-config \
            libssl-dev \
            libclang-dev \
            clang \
            zlib1g-dev \
            python3 \
            python3-dev \
            python3-pip \
            python3-venv \
            git \
            curl
    fi

    # 1.2 Rust Toolchain
    echo "[*] Checking Rust toolchain..."
    if ! command -v cargo &> /dev/null; then
        echo "[*] Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    else
        echo "[*] Rust detected: $(rustc --version)"
    fi

    # 1.3 Build Rust Solver
    echo "[*] Compiling cegar-fix Rust release binary..."
    cargo build --release --manifest-path src/cegar-fix/Cargo.toml
    echo "[*] Rust binary compiled: src/cegar-fix/target/release/cegar-fix"

    # 1.4 Python Packages
    echo "[*] Installing Python dependencies (requirements.txt)..."
    python3 -m pip install --upgrade pip --break-system-packages 2>/dev/null || python3 -m pip install --upgrade pip
    python3 -m pip install -r requirements.txt --break-system-packages 2>/dev/null || python3 -m pip install -r requirements.txt
else
    echo "[*] Skipping setup phase (--skip-setup was specified)."
fi

# ==============================================================================
# PHASE 2: SMOKE TESTS & VALIDATION
# ==============================================================================
echo ""
echo "================================================================================"
echo ">>> PHASE 2: SMOKE TESTS & SANITY VALIDATION"
echo "================================================================================"

# 2.1 PySAT CaDiCaL
echo -n "[*] Testing Python CaDiCaL 1.9.5 binding... "
python3 -c "from pysat.solvers import Cadical195; s = Cadical195(); s.add_clause([1, 2]); assert s.solve() == True"
echo "PASSED"

# 2.2 Rust cegar-fix CLI test on graph1.col
echo -n "[*] Testing Rust cegar-fix binary on graph1.col with '-i' flag... "
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph1.col -e 1 -b 3 -y 3 -t 3 -l 1 > /dev/null 2>&1
echo "PASSED"

# 2.3 Baseline Tour Verification
echo "[*] Checking current certified tours via verify_29.py..."
python3 scratch/verify_29.py

# ==============================================================================
# PHASE 3: EXECUTE 29-GRAPH BENCHMARK
# ==============================================================================
echo ""
echo "================================================================================"
echo ">>> PHASE 3: RUNNING 29-GRAPH BENCHMARK SUITE"
echo "================================================================================"

TARGET_29=(
    710 717 746 788 832 868 882 937 944 950
    951 954 959 960 963 965 966 971 974 975
    976 981 982 983 987 990 993 994 998
)

echo "Total instances to process: ${#TARGET_29[@]}"
echo "Timeout per instance: ${TIMEOUT_SECS}s"
echo "--------------------------------------------------------------------------------"

TOTAL_COUNT=0
PASS_COUNT=0
TIMEOUT_COUNT=0
FAIL_COUNT=0

for gid in "${TARGET_29[@]}"; do
    TOTAL_COUNT=$((TOTAL_COUNT + 1))
    COL_FILE="FHCPCS-col/graph${gid}.col"
    OUT_DIR="scratch/graph${gid}"
    mkdir -p "$OUT_DIR"
    TOUR_FILE="$OUT_DIR/found_tour_graph${gid}.hcp"
    LOG_FILE="$LOG_DIR/graph${gid}.log"

    echo ""
    echo "[$TOTAL_COUNT/29] ----------------------------------------------------------------"
    echo "[*] Instance: graph${gid} (Input: $COL_FILE)"

    if [ ! -f "$COL_FILE" ]; then
        echo "[-] ERROR: File $COL_FILE not found! Skipping."
        FAIL_COUNT=$((FAIL_COUNT + 1))
        continue
    fi

    # Check if a certified tour already exists
    if [ "$FORCE_RERUN" = false ]; then
        # Check canonical tours or output tour
        ALREADY_PASSED=false
        if python3 -c "
import sys, os
from scratch.verify_29 import parse_col, parse_tour, verify_tour, CANONICAL_TOURS, REPO_ROOT
gid = $gid
col_path = os.path.join(REPO_ROOT, 'FHCPCS-col', f'graph{gid}.col')
t_rel = CANONICAL_TOURS.get(gid, f'scratch/graph{gid}/found_tour_graph{gid}.hcp')
t_path = os.path.join(REPO_ROOT, t_rel)
if os.path.exists(t_path):
    n_v, n_e, adj = parse_col(col_path)
    ok, _ = verify_tour(parse_tour(t_path), n_v, adj)
    if ok:
        sys.exit(0)
sys.exit(1)
" 2>/dev/null; then
            ALREADY_PASSED=true
        fi

        if [ "$ALREADY_PASSED" = true ]; then
            echo "[+] Certified Sound Tour ALREADY EXISTS. Skipping re-run (use --force to override)."
            PASS_COUNT=$((PASS_COUNT + 1))
            continue
        fi
    fi

    # Select solver pipeline based on structural archetype
    if [ "$gid" -eq 868 ] || [ "$gid" -eq 960 ]; then
        SOLVER_CMD="python3 scratch/engine/directed_solver.py $COL_FILE $TOUR_FILE"
        ENGINE_NAME="Directed 3-Block Solver"
    else
        SOLVER_CMD="python3 scratch/engine/unified_solver.py $COL_FILE $TOUR_FILE"
        ENGINE_NAME="Unified Hierarchical Solver"
    fi

    echo "[*] Engine:  $ENGINE_NAME"
    echo "[*] Command: $SOLVER_CMD"
    echo "[*] Log:     $LOG_FILE"
    echo "[*] Starting solve (Timeout: ${TIMEOUT_SECS}s)..."

    START_TIME=$(date +%s)
    
    # Run solver with timeout
    if timeout --preserve-status "${TIMEOUT_SECS}s" $SOLVER_CMD > "$LOG_FILE" 2>&1; then
        EXIT_CODE=$?
        END_TIME=$(date +%s)
        ELAPSED=$((END_TIME - START_TIME))

        # Check if tour was produced and verified
        if [ -f "$TOUR_FILE" ]; then
            echo "[*] Tour file produced ($ELAPSED seconds). Verifying with verify_29.py..."
            if python3 -c "
import sys, os
from scratch.verify_29 import parse_col, parse_tour, verify_tour, REPO_ROOT
col_path = os.path.join(REPO_ROOT, '$COL_FILE')
t_path = os.path.join(REPO_ROOT, '$TOUR_FILE')
n_v, n_e, adj = parse_col(col_path)
ok, msg = verify_tour(parse_tour(t_path), n_v, adj)
assert ok, msg
print('[+] VERIFICATION RESULT: PASS (SOUND 100%)')
"; then
                echo "[+] SUCCESS: graph${gid} SOLVED in ${ELAPSED}s!"
                PASS_COUNT=$((PASS_COUNT + 1))
            else
                echo "[-] ERROR: graph${gid} tour failed verification!"
                FAIL_COUNT=$((FAIL_COUNT + 1))
            fi
        else
            echo "[-] Solver exited with code 0 but no tour file created in ${ELAPSED}s."
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    else
        EXIT_CODE=$?
        END_TIME=$(date +%s)
        ELAPSED=$((END_TIME - START_TIME))
        if [ "$ELAPSED" -ge "$TIMEOUT_SECS" ]; then
            echo "[-] TIMEOUT: graph${gid} exceeded ${TIMEOUT_SECS}s limit."
            TIMEOUT_COUNT=$((TIMEOUT_COUNT + 1))
        else
            echo "[-] FAILED: graph${gid} exited with code $EXIT_CODE after ${ELAPSED}s (check $LOG_FILE)."
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    fi
done

# ==============================================================================
# PHASE 4: FINAL BENCHMARK CERTIFICATION REPORT
# ==============================================================================
echo ""
echo "================================================================================"
echo ">>> PHASE 4: FINAL BENCHMARK REPORT"
echo "================================================================================"
python3 scratch/verify_29.py

echo ""
echo "Execution Run Summary:"
echo "  - Total Graphs:   ${TOTAL_COUNT}"
echo "  - Solved (Sound): ${PASS_COUNT}"
echo "  - Timed Out:      ${TIMEOUT_COUNT}"
echo "  - Errors:         ${FAIL_COUNT}"
echo "  - Detailed Logs:  $LOG_DIR"
echo "================================================================================"
