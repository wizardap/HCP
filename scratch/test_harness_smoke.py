#!/usr/bin/env python3
import subprocess
import os
import sys

def test_binary_ablation_flags():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    bin_path = os.path.join(repo_root, "src/cegar-fix/target/release/cegar-fix")
    assert os.path.exists(bin_path), f"Binary {bin_path} not compiled!"

    graph_path = os.path.join(repo_root, "FHCPCS-col/graph1.col")
    assert os.path.exists(graph_path), f"Graph file {graph_path} not found!"

    # Test C0, C1, C2, C3 on graph1.col with short timeout
    for mode in [0, 1, 2, 3]:
        out_tour = os.path.join(repo_root, f"scratch/smoke_c{mode}.hcp")
        cmd = [
            bin_path,
            "-i", graph_path,
            "--ablation", str(mode),
            "--timeout", "10.0",
            "--output-tour", out_tour
        ]
        res = subprocess.run(cmd, capture_output=True, text=True)
        assert res.returncode == 0, f"Ablation mode {mode} failed on graph1:\nSTDOUT: {res.stdout}\nSTDERR: {res.stderr}"
        assert "s SATISFIABLE" in res.stdout or "VALID" in res.stdout or "overall time" in res.stdout, f"Unexpected stdout for mode {mode}: {res.stdout}"
        if os.path.exists(out_tour):
            os.remove(out_tour)
        print(f"  [+] Mode C{mode} verified successfully on graph1.")

    print("Smoke test passed on all 4 ablation modes!")

if __name__ == '__main__':
    test_binary_ablation_flags()
