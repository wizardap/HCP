#!/usr/bin/env bash
# Script tu dong chay toan bo bo do thi FHCPCS-col voi PHIEN BAN SOTA CAI TIEN:
# - Restricted 3-opt (Candidate Graph)
# - CEGAR Hard Blocking Fallback
# - Partial MTZ Stall Injection
# Timeout: 1800s / testcase. Co do va in Total Benchmark Runtime.

set -e

# Nâng giới hạn Stack memory
ulimit -s unlimited 2>/dev/null || true
export RUST_MIN_STACK=536870912

CDIR="/home/ubuntu/HCP"
cd "$CDIR"

# Dam bao binary da duoc build release
echo "=== Building release binary of cegar-fix ==="
cd "$CDIR/src/cegar-fix" && cargo build --release
cd "$CDIR"

LOG_FILE="$CDIR/results_sota_3opt_mtz.log"
START_TIME=$(date +%s)
START_DATE=$(date)

echo "=== Start running FHCPCS-col (SOTA 3-Opt + Hard Fallback + Partial MTZ, Timeout 1800s) at $START_DATE ===" | tee "$LOG_FILE"

# Sap xep file theo thu tu so (graph1.col -> graph1001.col)
for f in $(ls -v FHCPCS-col/*.col); do
    echo "--------------------------------------------------" | tee -a "$LOG_FILE"
    echo "=== Processing $f at $(date) ===" | tee -a "$LOG_FILE"
    
    timeout 1800 ./src/cegar-fix/target/release/cegar-fix \
        -i "$f" \
        -e 1 \
        -b 3 \
        -y 0 \
        -t 3 \
        -l 1 \
        --three-opt 1 >> "$LOG_FILE" 2>&1 || echo "[WARNING] Graph $f exited with status code $?" | tee -a "$LOG_FILE"
done

END_TIME=$(date +%s)
END_DATE=$(date)
TOTAL_SECONDS=$((END_TIME - START_TIME))
HOURS=$((TOTAL_SECONDS / 3600))
MINUTES=$(((TOTAL_SECONDS % 3600) / 60))
SECONDS=$((TOTAL_SECONDS % 60))

echo "==================================================" | tee -a "$LOG_FILE"
echo "=== Completed all graphs at $END_DATE ===" | tee -a "$LOG_FILE"
echo "=== TOTAL BENCHMARK RUNTIME: ${HOURS}h ${MINUTES}m ${SECONDS}s (${TOTAL_SECONDS} seconds) ===" | tee -a "$LOG_FILE"
