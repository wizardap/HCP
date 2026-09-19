#!/usr/bin/env python3
"""
Automated 2-Phase Safe Timeout Workflow:
Phase 1: Scan graphs 1..400 using 1 worker and 10s timeout to identify timeouts.
Phase 2: Rerun all identified timeout graphs with 1 worker and 300s timeout.
"""

import os
import sys
import time
import json
import subprocess

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASE_JSON = os.path.join(REPO_ROOT, "scratch/batch_1001_results.json")
OUT_JSON = os.path.join(REPO_ROOT, "scratch/batch_400_300s_results.json")
WORKFLOW_LOG = os.path.join(REPO_ROOT, "scratch/safe_timeout_workflow.log")

def log(msg):
    t_str = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{t_str}] {msg}"
    print(line, flush=True)
    with open(WORKFLOW_LOG, "a") as f:
        f.write(line + "\n")

def main():
    log("================================================================================")
    log(">>> STARTING SAFE TIMEOUT WORKFLOW (1 WORKER TUẦN TỰ, SCAN 10s -> RERUN 300s)")
    log("================================================================================")

    # ---------------------------------------------------------
    # PHASE 1: Scan graphs 1..400 with 10s timeout
    # ---------------------------------------------------------
    log("[*] Phase 1: Scanning graphs 1..400 with 1 worker, timeout 10s...")
    cmd_phase1 = [
        sys.executable,
        os.path.join(REPO_ROOT, "scratch/batch_1001_runner.py"),
        "--start", "1",
        "--end", "400",
        "--workers", "1",
        "--timeout", "10"
    ]

    p1 = subprocess.run(cmd_phase1, cwd=REPO_ROOT)
    if p1.returncode != 0:
        log(f"[-] Phase 1 exited with error code {p1.returncode}")
        sys.exit(p1.returncode)

    log("[+] Phase 1 scanning complete!")

    # Check timeouts
    if not os.path.exists(BASE_JSON):
        log(f"[-] ERROR: {BASE_JSON} not found after Phase 1!")
        sys.exit(1)

    with open(BASE_JSON, "r") as f:
        base_results = json.load(f)

    timeouts = [d for d in base_results if d.get("gid", 0) <= 400 and d.get("status") == "TIMEOUT"]
    log(f"[*] Found {len(timeouts)} timeout graphs out of {len(base_results)} processed instances (under graph400).")

    if not timeouts:
        log("[*] No timeouts found! Workflow complete.")
        return

    # ---------------------------------------------------------
    # PHASE 2: Rerun all timeouts with 300s timeout (1 worker)
    # ---------------------------------------------------------
    log("================================================================================")
    log(f"[*] Phase 2: Rerunning {len(timeouts)} timeout graphs with 1 worker, timeout 300s...")
    log("================================================================================")

    cmd_phase2 = [
        sys.executable,
        os.path.join(REPO_ROOT, "scratch/batch_rerun_timeouts_300s.py")
    ]

    p2 = subprocess.run(cmd_phase2, cwd=REPO_ROOT)
    if p2.returncode != 0:
        log(f"[-] Phase 2 exited with code {p2.returncode}")
        sys.exit(p2.returncode)

    log("[+] Phase 2 rerun complete!")

    if os.path.exists(OUT_JSON):
        with open(OUT_JSON, "r") as f:
            final_res = json.load(f)
        sat_solved = sum(1 for d in final_res if d.get("status") == "SAT_VERIFIED")
        still_timeout = sum(1 for d in final_res if d.get("status") == "TIMEOUT")
        log(f"[*] Final Results for 300s Rerun: Total={len(final_res)}, Solved={sat_solved}, Timeout={still_timeout}")

    log("================================================================================")
    log("[*] SAFE TIMEOUT WORKFLOW COMPLETED SUCCESSFULLY.")
    log("================================================================================")

if __name__ == "__main__":
    main()
