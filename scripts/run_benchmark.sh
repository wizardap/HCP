#!/usr/bin/env bash
# ==============================================================================
# Script: scripts/run_benchmark.sh
# Purpose: Distributed / Multi-Machine HCP Benchmark Runner (1001 Graphs)
# Specifications:
#   - Resource Limits: 1800s wall/cpu time limit, 7GB RAM limit (via runlim)
#   - Core Throttling: Leaves 1 CPU core free to prevent OS freeze / crash
#   - Crash-Safety: Appends to CSV after every testcase & syncs to disk
#   - Resume Capability: Automatically skips already completed graphs
#   - Solvetime Accounting: Records solving time before verification starts
#   - Machine Splitting: Configurable start and end graph IDs
#       * Machine 1: ./scripts/run_benchmark.sh 1 700
#       * Machine 2: ./scripts/run_benchmark.sh 701 1001
# ==============================================================================

set -euo pipefail

# Ensure Cargo/Rust environment is sourced if present
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi
if [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# --- Arguments & Defaults ---
START_ID="${1:-1}"
END_ID="${2:-1001}"
OUTPUT_DIR="${3:-benchmark_results}"

# --- Paths Resolution ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GRAPH_DIR="$PROJECT_ROOT/FHCPCS-col"
SOLVER_BIN="$PROJECT_ROOT/src/cegar-fix/target/release/cegar-fix"
RUNLIM_BIN=""

# Create output subdirectories
CSV_FILE="$OUTPUT_DIR/benchmark_results.csv"
LOG_DIR="$OUTPUT_DIR/logs"
RUNLIM_DIR="$OUTPUT_DIR/runlim"
TOUR_DIR="$OUTPUT_DIR/tours"
mkdir -p "$LOG_DIR" "$RUNLIM_DIR" "$TOUR_DIR"

# --- Function: Locate or Build runlim ---
locate_runlim() {
    if command -v runlim >/dev/null 2>&1; then
        RUNLIM_BIN="$(command -v runlim)"
        return 0
    fi
    if [ -x "/usr/local/bin/runlim" ]; then
        RUNLIM_BIN="/usr/local/bin/runlim"
        return 0
    fi
    if [ -x "$PROJECT_ROOT/bin/runlim" ]; then
        RUNLIM_BIN="$PROJECT_ROOT/bin/runlim"
        return 0
    fi

    echo "[INFO] runlim not found in PATH or standard locations. Compiling runlim..."
    mkdir -p "$PROJECT_ROOT/bin"
    local BUILD_DIR
    BUILD_DIR=$(mktemp -d)
    if curl -s -L https://github.com/arminbiere/runlim/archive/refs/heads/master.tar.gz -o "$BUILD_DIR/runlim.tar.gz"; then
        tar -xzf "$BUILD_DIR/runlim.tar.gz" -C "$BUILD_DIR"
        (cd "$BUILD_DIR/runlim-master" && ./configure.sh && make)
        if [ -f "$BUILD_DIR/runlim-master/runlim" ]; then
            cp "$BUILD_DIR/runlim-master/runlim" "$PROJECT_ROOT/bin/runlim"
            RUNLIM_BIN="$PROJECT_ROOT/bin/runlim"
            rm -rf "$BUILD_DIR"
            echo "[INFO] Successfully compiled and installed runlim to: $RUNLIM_BIN"
            return 0
        fi
    fi
    rm -rf "$BUILD_DIR"
    echo "[ERROR] Failed to compile runlim. Please ensure gcc and make are installed."
    exit 1
}

# --- Function: Ensure Solver Binary is Built ---
ensure_solver() {
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi
    if [ -d "$HOME/.cargo/bin" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi

    if command -v cargo >/dev/null 2>&1; then
        echo "[INFO] Ensuring solver binary is built and up-to-date with cargo..."
        (cd "$PROJECT_ROOT/src/cegar-fix" && cargo build --release)
    elif [ -x "$SOLVER_BIN" ]; then
        echo "[WARNING] 'cargo' command not found, but solver binary exists at: $SOLVER_BIN. Proceeding."
    else
        echo "[ERROR] 'cargo' is not installed or not in PATH, and solver binary does not exist at: $SOLVER_BIN"
        echo "[INFO] Please install Rust using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        echo "       and then run: source \"\$HOME/.cargo/env\""
        exit 1
    fi

    if [ ! -x "$SOLVER_BIN" ]; then
        echo "[ERROR] Solver binary at $SOLVER_BIN is not executable."
        exit 1
    fi
}

# --- Function: CPU Core Throttling (Leave 1 Core Free) ---
setup_cpu_cores() {
    local TOTAL_CPUS
    TOTAL_CPUS=$(nproc)
    if [ "$TOTAL_CPUS" -gt 1 ]; then
        USABLE_CPUS=$((TOTAL_CPUS - 1))
        CPU_AFFINITY="0-$((USABLE_CPUS - 1))"
    else
        USABLE_CPUS=1
        CPU_AFFINITY="0"
    fi

    export RAYON_NUM_THREADS="$USABLE_CPUS"
    export OMP_NUM_THREADS="$USABLE_CPUS"

    if command -v taskset >/dev/null 2>&1; then
        TASKSET_PREFIX="taskset -c $CPU_AFFINITY"
    else
        TASKSET_PREFIX=""
    fi
}

# --- Function: Check If Graph Was Already Completed ---
is_graph_completed() {
    local gid="$1"
    if [ -f "$CSV_FILE" ]; then
        awk -F',' -v target="$gid" '
            NR > 1 && $1 == target && ($2 == "SAT" || $2 == "UNSAT" || $2 == "TIMEOUT" || $2 == "MEMOUT") {
                found = 1;
                exit 0;
            }
            END {
                exit !found;
            }
        ' "$CSV_FILE"
    else
        return 1
    fi
}

# --- Main Setup ---
locate_runlim
ensure_solver
setup_cpu_cores

# Initialize CSV if not present
if [ ! -f "$CSV_FILE" ]; then
    echo "graph_id,status,solve_time_sec,verify_time_sec,total_wall_sec,memory_mb,exit_code,timestamp" > "$CSV_FILE"
    sync
fi

echo "=============================================================================="
echo " HCP Benchmark Runner Initialized"
echo " Target Range  : Graph $START_ID to Graph $END_ID"
echo " Solver Binary : $SOLVER_BIN"
echo " Runlim Binary : $RUNLIM_BIN"
echo " Time Limit    : 1800 seconds (real/wall-clock limit)"
echo " Memory Limit  : 7168 MB (7 GB)"
echo " CPU Allocation: $USABLE_CPUS cores allocated (1 core reserved for system safety)"
if [ -n "$TASKSET_PREFIX" ]; then
echo " CPU Affinity  : cores $CPU_AFFINITY"
fi
echo " Output Directory: $OUTPUT_DIR"
echo " Results CSV   : $CSV_FILE"
echo "=============================================================================="

# --- Benchmark Execution Loop ---
TOTAL_COUNT=$((END_ID - START_ID + 1))
PROCESSED_COUNT=0
SAT_COUNT=0
UNSAT_COUNT=0
TIMEOUT_COUNT=0
MEMOUT_COUNT=0
ERROR_COUNT=0

for ((gid = START_ID; gid <= END_ID; gid++)); do
    GRAPH_PATH="$GRAPH_DIR/graph${gid}.col"
    PROCESSED_COUNT=$((PROCESSED_COUNT + 1))

    if [ ! -f "$GRAPH_PATH" ]; then
        echo "[WARNING] [$PROCESSED_COUNT/$TOTAL_COUNT] File not found: $GRAPH_PATH (skipping)"
        continue
    fi

    # Check for resume capability
    if is_graph_completed "$gid"; then
        EXISTING_STATUS=$(awk -F',' -v target="$gid" 'NR > 1 && $1 == target {print $2; exit}' "$CSV_FILE" 2>/dev/null || true)
        EXISTING_TIME=$(awk -F',' -v target="$gid" 'NR > 1 && $1 == target {print $3; exit}' "$CSV_FILE" 2>/dev/null || true)
        echo "[RESUME] [$PROCESSED_COUNT/$TOTAL_COUNT] Graph $gid already completed (Status: $EXISTING_STATUS, Solve Time: ${EXISTING_TIME}s). Skipping."
        continue
    fi

    echo "------------------------------------------------------------------------------"
    echo "[RUNNING] [$PROCESSED_COUNT/$TOTAL_COUNT] Starting Graph $gid (File: graph${gid}.col)..."

    SOLVER_LOG="$LOG_DIR/graph_${gid}.log"
    RUNLIM_LOG="$RUNLIM_DIR/graph_${gid}.runlim"
    TOUR_FILE="$TOUR_DIR/graph_${gid}.hcp"

    EXIT_CODE=0
    # Execute runlim with real time limit 1800s, space limit 7168 MB (7GB)
    $RUNLIM_BIN -r 1800 -s 7168 -o "$RUNLIM_LOG" \
        $TASKSET_PREFIX "$SOLVER_BIN" \
        -i "$GRAPH_PATH" \
        --timeout 1800 \
        -o "$TOUR_FILE" > "$SOLVER_LOG" 2>&1 || EXIT_CODE=$?

    # --- Metrics Extraction from runlim ---
    RUNLIM_STATUS=$(grep "^\[runlim\] status:" "$RUNLIM_LOG" 2>/dev/null | awk '{$1=""; print $0}' | sed 's/^[ \t]*//' || true)
    if [ -z "$RUNLIM_STATUS" ]; then RUNLIM_STATUS="unknown"; fi
    RUNLIM_REAL=$(grep "^\[runlim\] real:" "$RUNLIM_LOG" 2>/dev/null | awk '{print $3}' || true)
    if [ -z "$RUNLIM_REAL" ]; then RUNLIM_REAL="0.0"; fi
    RUNLIM_SPACE=$(grep "^\[runlim\] space:" "$RUNLIM_LOG" 2>/dev/null | awk '{print $3}' || true)
    if [ -z "$RUNLIM_SPACE" ]; then RUNLIM_SPACE="0"; fi

    # --- Status and Timing Determination ---
    STATUS="UNKNOWN"
    SOLVE_TIME="0.0"
    VERIFY_TIME="0.0"

    # Check if runlim triggered space/memory limit
    if echo "$RUNLIM_STATUS" | grep -qiE "space|memory"; then
        STATUS="MEMOUT"
        SOLVE_TIME="$RUNLIM_REAL"
        MEMOUT_COUNT=$((MEMOUT_COUNT + 1))
    # Check if runlim triggered time limit
    elif echo "$RUNLIM_STATUS" | grep -qiE "time"; then
        STATUS="TIMEOUT"
        SOLVE_TIME="$RUNLIM_REAL"
        TIMEOUT_COUNT=$((TIMEOUT_COUNT + 1))
    # Check solver log for SAT
    elif grep -q "s SATISFIABLE" "$SOLVER_LOG"; then
        STATUS="SAT"
        SAT_COUNT=$((SAT_COUNT + 1))
        # Extract solve time before verification
        EXTRACTED_SOLVE=$(grep "^solve_time_sec:" "$SOLVER_LOG" 2>/dev/null | awk '{print $2}' || true)
        if [ -n "$EXTRACTED_SOLVE" ]; then
            SOLVE_TIME="$EXTRACTED_SOLVE"
        else
            # Fallback to overall time if solve_time_sec not present
            OVERALL_FALLBACK=$(grep "^overall time = " "$SOLVER_LOG" 2>/dev/null | awk '{print $4}' | sed 's/s$//' || true)
            if [ -n "$OVERALL_FALLBACK" ]; then
                SOLVE_TIME="$OVERALL_FALLBACK"
            else
                SOLVE_TIME="$RUNLIM_REAL"
            fi
        fi
        EXTRACTED_VERIFY=$(grep "^verify_time_sec:" "$SOLVER_LOG" 2>/dev/null | awk '{print $2}' || true)
        if [ -n "$EXTRACTED_VERIFY" ]; then
            VERIFY_TIME="$EXTRACTED_VERIFY"
        fi
    # Check solver log for UNSAT
    elif grep -q "s UNSATISFIABLE" "$SOLVER_LOG"; then
        STATUS="UNSAT"
        UNSAT_COUNT=$((UNSAT_COUNT + 1))
        SOLVE_TIME="$RUNLIM_REAL"
    # Check for solver timeout string or 1800s wall time
    elif grep -qi "TIMEOUT" "$SOLVER_LOG" || (( $(echo "$RUNLIM_REAL >= 1800.0" | bc -l 2>/dev/null || echo 0) )); then
        STATUS="TIMEOUT"
        SOLVE_TIME="$RUNLIM_REAL"
        TIMEOUT_COUNT=$((TIMEOUT_COUNT + 1))
    # Non-zero exit code or panic
    elif [ "$EXIT_CODE" -ne 0 ] || grep -qi "panic" "$SOLVER_LOG"; then
        STATUS="ERROR"
        SOLVE_TIME="$RUNLIM_REAL"
        ERROR_COUNT=$((ERROR_COUNT + 1))
    else
        STATUS="UNKNOWN"
        SOLVE_TIME="$RUNLIM_REAL"
    fi

    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # --- Immediate Persistence (Crash-Proof) ---
    echo "${gid},${STATUS},${SOLVE_TIME},${VERIFY_TIME},${RUNLIM_REAL},${RUNLIM_SPACE},${EXIT_CODE},${TIMESTAMP}" >> "$CSV_FILE"
    sync

    echo "[COMPLETED] Graph $gid | Status: $STATUS | Solve Time: ${SOLVE_TIME}s | Verify: ${VERIFY_TIME}s | Wall: ${RUNLIM_REAL}s | Peak Mem: ${RUNLIM_SPACE} MB"
done

echo "=============================================================================="
echo " Benchmark Run Finished (Graphs $START_ID to $END_ID)"
echo " Results saved to: $CSV_FILE"
echo " Summary: SAT: $SAT_COUNT | UNSAT: $UNSAT_COUNT | TIMEOUT: $TIMEOUT_COUNT | MEMOUT: $MEMOUT_COUNT | ERROR: $ERROR_COUNT"
echo "=============================================================================="
