#!/usr/bin/env bash
# ==============================================================================
# Script: scripts/run_machine2.sh
# Purpose: Execute Benchmark Suite on Machine 2 (Default: Graphs 701 to 1001)
# Usage:
#   ./scripts/run_machine2.sh                    # Runs default: 701 to 1001
#   ./scripts/run_machine2.sh <START> <END> [DIR]# Custom range/testing
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default: 701 to 1001 into benchmark_machine2
START_ID="${1:-701}"
END_ID="${2:-1001}"
OUTPUT_DIR="${3:-benchmark_machine2}"

echo "[MACHINE 2] Initializing benchmark for Graphs $START_ID to $END_ID..."
echo "[MACHINE 2] Output directory: $OUTPUT_DIR"

exec "$SCRIPT_DIR/run_benchmark.sh" "$START_ID" "$END_ID" "$OUTPUT_DIR"
