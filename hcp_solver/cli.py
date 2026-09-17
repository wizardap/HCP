"""
Command-line interface for HCP Solver.
"""

import argparse
import sys
import time
from .router import HCPRouter

def main():
    parser = argparse.ArgumentParser(
        description="Unified Lean Solver for 11 HCP Challenge Graphs"
    )
    parser.add_argument(
        "input",
        type=str,
        help="Path to DIMACS .col graph file (e.g. FHCPCS-col/graph746.col)"
    )
    parser.add_argument(
        "-o", "--output",
        type=str,
        default=None,
        help="Optional path to output .hcp tour file"
    )
    parser.add_argument(
        "--no-verify",
        action="store_true",
        help="Skip strict mathematical verification against raw graph (not recommended)"
    )

    args = parser.parse_args()

    t0 = time.time()
    try:
        tour = HCPRouter.solve_file(
            col_path=args.input,
            out_tour_path=args.output,
            verify=not args.no_verify
        )
        elapsed = time.time() - t0
        print(f"[*] Done in {elapsed:.3f}s. Tour length: {len(tour)} vertices.")
        sys.exit(0)
    except Exception as e:
        print(f"[-] Solver failed: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
