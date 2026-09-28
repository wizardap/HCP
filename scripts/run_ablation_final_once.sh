#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_dir="${HCP_ABLATION_RUN_DIR:-$repo_root/benchmark-runs/ablation-final-once-1001}"
binary="$repo_root/src/cegar-fix/target/release/cegar-fix"
graphs_dir="${HCP_GRAPHS_DIR:-$repo_root/FHCPCS-col}"
conditions="full,no-dispatch,no-decomposition,no-two-cut"
seed=1
timeout=1800

mkdir -p "$run_dir/raw-logs" "$run_dir/tours"

graph_count="$(find "$graphs_dir" -maxdepth 1 -type f -name 'graph*.col' | wc -l | tr -d ' ')"
if [[ "$graph_count" != "1001" ]]; then
  echo "Expected exactly 1001 graph*.col files in $graphs_dir; found $graph_count" >&2
  exit 2
fi

if ! git -C "$repo_root" diff --quiet -- || \
   ! git -C "$repo_root" diff --cached --quiet --; then
  echo "Tracked source files have uncommitted changes. Commit or stash them before the final run." >&2
  exit 2
fi

cargo build --release --locked --manifest-path "$repo_root/src/cegar-fix/Cargo.toml"

REPO_ROOT="$repo_root" BINARY="$binary" GRAPHS_DIR="$graphs_dir" RUN_DIR="$run_dir" \
CONDITIONS="$conditions" SEED="$seed" TIMEOUT="$timeout" python3 - <<'PY'
import hashlib
import json
import os
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path

repo = Path(os.environ["REPO_ROOT"])
binary = Path(os.environ["BINARY"])
run_dir = Path(os.environ["RUN_DIR"])
graphs_dir = Path(os.environ["GRAPHS_DIR"]).resolve()
metadata_path = run_dir / "metadata.json"
results_path = run_dir / "results.jsonl"

def command_output(command):
    try:
        return subprocess.check_output(command, cwd=repo, text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return None

graph_paths = list(graphs_dir.glob("graph*.col"))
expected_names = {f"graph{i}.col" for i in range(1, 1002)}
actual_names = {path.name for path in graph_paths}
if actual_names != expected_names:
    raise SystemExit(
        "Dataset mismatch: "
        f"missing={sorted(expected_names - actual_names)}, "
        f"unexpected={sorted(actual_names - expected_names)}"
    )
graph_paths.sort(key=lambda path: int(path.stem[5:]))

manifest_lines = []
for path in graph_paths:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest_lines.append(f"{digest}  {path.name}")
manifest_text = "\n".join(manifest_lines) + "\n"

now = datetime.now(timezone.utc).isoformat()
metadata = {
    "started_at_utc": now,
    "protocol": "final-single-run",
    "protocol_document": "docs/research/experiments/2026-09-28-final-single-run-ablation-protocol.md",
    "instances": 1001,
    "conditions": os.environ["CONDITIONS"].split(","),
    "seed": int(os.environ["SEED"]),
    "timeout_seconds": int(os.environ["TIMEOUT"]),
    "runs_expected": 1001 * len(os.environ["CONDITIONS"].split(",")),
    "execution": "one process at a time; cyclically balanced condition order; up to three solver-internal threads",
    "git_commit": command_output(["git", "rev-parse", "HEAD"]),
    "git_branch": command_output(["git", "branch", "--show-current"]),
    "git_status_porcelain": command_output(["git", "status", "--porcelain", "--untracked-files=no"]),
    "binary": str(binary.resolve()),
    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "graphs_dir": str(graphs_dir),
    "dataset_manifest": str((run_dir / "instances.sha256").resolve()),
    "dataset_manifest_sha256": hashlib.sha256(manifest_text.encode()).hexdigest(),
    "platform": platform.platform(),
    "machine": platform.machine(),
    "processor": platform.processor() or command_output(["sysctl", "-n", "machdep.cpu.brand_string"]),
    "logical_cpus": os.cpu_count(),
    "memory_bytes": command_output(["sysctl", "-n", "hw.memsize"]),
    "python": platform.python_version(),
    "rustc": command_output(["rustc", "--version"]),
    "cargo": command_output(["cargo", "--version"]),
}

if results_path.exists() and results_path.stat().st_size:
    if not metadata_path.exists():
        raise SystemExit("Cannot resume: results.jsonl exists without metadata.json")
    previous = json.loads(metadata_path.read_text())
    frozen_keys = (
        "conditions", "seed", "timeout_seconds", "git_commit", "binary_sha256",
        "graphs_dir", "dataset_manifest_sha256",
    )
    mismatches = [key for key in frozen_keys if previous.get(key) != metadata.get(key)]
    if mismatches:
        raise SystemExit("Cannot resume: frozen metadata differs for " + ", ".join(mismatches))
    metadata["started_at_utc"] = previous["started_at_utc"]
    metadata["resume_times_utc"] = previous.get("resume_times_utc", []) + [now]

(run_dir / "instances.sha256").write_text(manifest_text)
metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")
PY

export LC_ALL=C
export OMP_NUM_THREADS=1
export RAYON_NUM_THREADS=3
export PYTHONUNBUFFERED=1

runner=(
  python3 "$repo_root/tools/benchmark_runner_ablation.py"
  --binary "$binary"
  --graphs-dir "$graphs_dir"
  --conditions "$conditions"
  --seeds "$seed"
  --timeout "$timeout"
  --output "$run_dir/results.jsonl"
  --tour-dir "$run_dir/tours"
  --raw-log-dir "$run_dir/raw-logs"
  --balanced-condition-order
)

if command -v caffeinate >/dev/null 2>&1; then
  caffeinate -ims "${runner[@]}" 2>&1 | tee -a "$run_dir/progress.log"
else
  "${runner[@]}" 2>&1 | tee -a "$run_dir/progress.log"
fi

python3 "$repo_root/tools/analyze_ablation_results.py" \
  --input "$run_dir/results.jsonl" \
  --timeout "$timeout" \
  --expected-seeds 1 \
  --cactus-out "$run_dir/cactus.csv" \
  | tee "$run_dir/report.md"

RESULTS="$run_dir/results.jsonl" METADATA="$run_dir/metadata.json" python3 - <<'PY'
import json
import os
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

results_path = Path(os.environ["RESULTS"])
metadata_path = Path(os.environ["METADATA"])
records = [json.loads(line) for line in results_path.read_text().splitlines() if line.strip()]
conditions = ("full", "no-dispatch", "no-decomposition", "no-two-cut")
expected_features = {
    "full": "name=full seed=1 dispatch=true series=true two_cut=true repair=true directed_cubic=true alternating_workers=3",
    "no-dispatch": "name=no-dispatch seed=1 dispatch=false series=true two_cut=true repair=true directed_cubic=false alternating_workers=3",
    "no-decomposition": "name=no-decomposition seed=1 dispatch=false series=false two_cut=false repair=true directed_cubic=false alternating_workers=3",
    "no-two-cut": "name=no-two-cut seed=1 dispatch=true series=true two_cut=false repair=true directed_cubic=true alternating_workers=3",
}
expected_keys = {(f"graph{i}", condition, 1) for i in range(1, 1002) for condition in conditions}
actual_keys = {(r["graph"], r["condition"], int(r["seed"])) for r in records}

errors = []
if len(records) != 4004:
    errors.append(f"expected 4004 records, found {len(records)}")
if actual_keys != expected_keys:
    errors.append(
        f"key mismatch: missing={len(expected_keys - actual_keys)}, "
        f"unexpected={len(actual_keys - expected_keys)}"
    )
if any(float(r["timeout_limit"]) != 1800.0 for r in records):
    errors.append("at least one record has a non-1800-second cutoff")
bad_features = [
    r for r in records
    if r.get("feature_vector") != expected_features.get(r.get("condition"))
]
if bad_features:
    errors.append(f"{len(bad_features)} records have a missing or unexpected feature vector")
if any(r.get("status") == "SATISFIABLE" and r.get("verified") is not True for r in records):
    errors.append("at least one SAT result lacks an independently verified tour")
if any(r.get("status") == "SATISFIABLE" and not Path(r.get("tour_path", "")).is_file() for r in records):
    errors.append("at least one verified SAT record is missing its archived tour")
if any(
    not Path(r.get(field, "")).is_file()
    for r in records
    for field in ("stdout_path", "stderr_path")
):
    errors.append("at least one record is missing its raw stdout/stderr log")
if any(float(r.get("cpu_total_time", -1)) < 0 for r in records):
    errors.append("at least one record is missing valid aggregate CPU time")
unsat_runs = [r for r in records if r.get("status") == "UNSATISFIABLE"]
if unsat_runs:
    errors.append(f"{len(unsat_runs)} runs reported UNSAT on known-Hamiltonian instances")
bad_runs = [r for r in records if r.get("status") in ("ERROR", "UNKNOWN")]
if bad_runs:
    errors.append(f"{len(bad_runs)} runs ended in ERROR/UNKNOWN")

metadata = json.loads(metadata_path.read_text())
if any(r.get("binary_sha256") != metadata["binary_sha256"] for r in records):
    errors.append("at least one record has a different binary hash")
if any(r.get("git_commit") != metadata["git_commit"] for r in records):
    errors.append("at least one record has a different Git revision")
metadata["completed_at_utc"] = datetime.now(timezone.utc).isoformat()
metadata["records"] = len(records)
metadata["status_counts"] = dict(Counter(r["status"] for r in records))
metadata["postflight_errors"] = errors
metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")

if errors:
    raise SystemExit("Postflight validation failed: " + "; ".join(errors))
print("Postflight validation passed: 4004 unique, matched, independently checked runs.")
PY

echo "Final report: $run_dir/report.md"
echo "Raw results: $run_dir/results.jsonl"
