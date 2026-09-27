#!/usr/bin/env bash
# ==============================================================================
# Script: scripts/run_machine1.sh
# Purpose: Execute Benchmark Suite on Machine 1 (Default: Graphs 1 to 700)
# Usage:
#   ./scripts/run_machine1.sh                    # Runs default: 1 to 700
#   ./scripts/run_machine1.sh <START> <END> [DIR]# Custom range/testing
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default: 1 to 700 into benchmark_machine1
START_ID="${1:-1}"
END_ID="${2:-700}"
OUTPUT_DIR="${3:-benchmark_machine1}"

echo "[MACHINE 1] Initializing benchmark for Graphs $START_ID to $END_ID..."
echo "[MACHINE 1] Output directory: $OUTPUT_DIR"

exec "$SCRIPT_DIR/run_benchmark.sh" "$START_ID" "$END_ID" "$OUTPUT_DIR"
