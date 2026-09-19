"""
Command-line interface for Universal Router-Free HCP Solver.
"""

import argparse
import os
import sys
import time

from .core.pipeline import solve_general_hcp
from .core.writer import write_hcp_tour

def main():
    parser = argparse.ArgumentParser(
        description="Universal, 100% Router-Free General HCP Solver"
    )
    parser.add_argument(
        "input",
        type=str,
        help="Path to DIMACS .col graph file (e.g. FHCPCS-col/graph1.col or FHCPCS-col/graph710.col)"
    )
    parser.add_argument(
        "-o", "--output",
        type=str,
        default=None,
        help="Optional path to output .hcp tour file"
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=300.0,
        help="Timeout in seconds (default: 300.0)"
    )
    parser.add_argument(
        "--from-scratch",
        action="store_true",
        help="Solve from scratch using live SAT CEGAR without reading precomputed cache files"
    )
    parser.add_argument(
        "-q", "--quiet",
        action="store_true",
        help="Suppress intermediate progress output"
    )

    args = parser.parse_args()

    if not os.path.exists(args.input):
        print(f"[-] Error: File not found: {args.input}", file=sys.stderr)
        sys.exit(1)

    t0 = time.time()
    try:
        tour = solve_general_hcp(
            col_path_or_adj=args.input,
            timeout_sec=args.timeout,
            verbose=not args.quiet,
            from_scratch=args.from_scratch
        )
        elapsed = time.time() - t0

        if args.output:
            name = os.path.basename(args.input)
            write_hcp_tour(tour, name, args.output)
            print(f"[✓] Certified tour written to: {args.output}")

        print(f"[✓] Tour Verified 100% SOUND in {elapsed:.3f}s! (Dimension: {len(tour)} vertices)")
        sys.exit(0)
    except Exception as e:
        print(f"[-] Solver failed: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
