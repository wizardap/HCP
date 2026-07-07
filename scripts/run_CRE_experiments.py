import os
import sys
import time
import subprocess
import re

CRE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "refs", "ChineseRemainderEncoding")
CADICAL = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "refs", "cadical", "build", "cadical")
HCP_ENCODE = os.path.join(CRE_DIR, "hcp-encode")
HCP_DECODE = os.path.join(CRE_DIR, "hcp-decode")
GRAPHS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "graphs")

def parse_args():
    import argparse
    parser = argparse.ArgumentParser(description="Run CRE experiments on HCP graphs")
    parser.add_argument("-c", "--cycle", type=int, default=None,
                        help="Cycle length (omit to compute automatically per graph like run-wo-cycle.sh)")
    parser.add_argument("-t", "--time-limit", type=int, default=600,
                        help="Solver time limit per graph in seconds (default: 600)")
    parser.add_argument("-g", "--graph", type=str, default=None,
                        help="Run only graphs matching this substring (e.g. '48' or 'graph48')")
    return parser.parse_args()

def main():
    args = parse_args()
    script_dir = os.path.dirname(os.path.abspath(__file__))
    files = []
    for root, _, filenames in os.walk(GRAPHS_DIR):
        for f in filenames:
            if f.endswith(".edge"):
                rel_path = os.path.relpath(os.path.join(root, f), GRAPHS_DIR)
                files.append(rel_path)

    def get_sort_key(filename):
        subdir = os.path.dirname(filename)
        base = os.path.basename(filename)
        match = re.search(r'\d+', base)
        num = int(match.group()) if match else float('inf')
        return (subdir, num, filename)
    files.sort(key=get_sort_key)

    if args.graph:
        files = [f for f in files if args.graph in f]
        if not files:
            print(f"No graphs matching '{args.graph}'")
            return

    print(f"{'Graph':<35} | {'Vertices':<9} | {'Cycle':<7} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}")
    print("-" * 120)

    log_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "CRE_sol.log")
    with open(log_file, "w") as log:
        log.write(f"{'Graph':<35} | {'Vertices':<9} | {'Cycle':<7} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}\n")
        log.write("-" * 120 + "\n")

        for file in files:
            graph_path = os.path.join(GRAPHS_DIR, file)

            # Extract vertex count
            nNode = 0
            try:
                with open(graph_path, "r") as f:
                    for line in f:
                        if line.startswith("p edge"):
                            parts = line.split()
                            nNode = int(parts[2])
                            break
            except Exception:
                pass

            cmd = [HCP_ENCODE, graph_path]
            if args.cycle is not None:
                cmd.append(str(args.cycle))

            temp_cnf = os.path.join(script_dir, "temp_cre.cnf")
            temp_sat = os.path.join(script_dir, "temp_cre.sat")

            # Step 1: Encode with CRE
            n_vars = 0
            n_clauses = 0
            try:
                proc = subprocess.run(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    check=True
                )
                raw_out = proc.stdout.decode()

                # Parse the raw output lines and rewrite header
                lines = raw_out.splitlines()
                cnf_lines = []
                actual_n_clauses = 0
                header_index = -1

                for i, line in enumerate(lines):
                    stripped = line.strip()
                    if stripped.startswith("p cnf"):
                        header_index = i
                        parts = stripped.split()
                        n_vars = int(parts[2])
                    elif stripped and not stripped.startswith("c"):
                        actual_n_clauses += 1
                    cnf_lines.append(line)

                if header_index != -1:
                    cnf_lines[header_index] = f"p cnf {n_vars} {actual_n_clauses}"
                    n_clauses = actual_n_clauses

                with open(temp_cnf, "w") as f:
                    f.write("\n".join(cnf_lines) + "\n")

            except Exception as e:
                if os.path.exists(temp_cnf):
                    os.remove(temp_cnf)
                cycle_str = str(args.cycle) if args.cycle is not None else "auto"
                msg = f"{file:<35} | {nNode:<9} | {cycle_str:<7} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'EncodeErr':<12} | {'No':<10}"
                print(msg)
                log.write(msg + "\n")
                log.flush()
                continue

            # Step 2: Solve with cadical
            t_start = time.time()
            status = "Unknown"
            solve_time = 0.0

            try:
                proc = subprocess.run(
                    [CADICAL, temp_cnf, "-t", str(args.time_limit), "-w", temp_sat],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=args.time_limit + 10
                )
                t_end = time.time()
                solve_time = t_end - t_start

                is_sat = False
                is_unsat = False
                with open(temp_sat, "r") as f:
                    for line in f:
                        if "UNSATISFIABLE" in line:
                            is_unsat = True
                        elif "SATISFIABLE" in line:
                            is_sat = True

                if is_sat or proc.returncode == 10:
                    status = "SAT"
                elif is_unsat or proc.returncode == 20:
                    status = "UNSAT"
                else:
                    status = "Timeout"
            except subprocess.TimeoutExpired:
                solve_time = float(args.time_limit)
                status = "Timeout"
            except Exception as e:
                status = "SolveErr"

            # Step 3: Decode/Verify if SAT
            verified = "No"
            if status == "SAT":
                try:
                    dec_proc = subprocess.run(
                        [HCP_DECODE, graph_path, temp_sat],
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        check=True
                    )
                    output = (dec_proc.stdout + dec_proc.stderr).decode()
                    if "VERIFIED" in output:
                        verified = "Yes"
                    else:
                        verified = "Failed"
                except Exception as e:
                    verified = "DecErr"
            elif status == "UNSAT":
                verified = "N/A"
            else:
                verified = "N/A"

            for f_tmp in [temp_cnf, temp_sat]:
                if os.path.exists(f_tmp):
                    os.remove(f_tmp)

            cycle_str = str(args.cycle) if args.cycle is not None else "auto"
            msg = f"{file:<35} | {nNode:<9} | {cycle_str:<7} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {status:<12} | {verified:<10}"
            print(msg)
            log.write(msg + "\n")
            log.flush()

    print(f"\nAll CRE experiments finished. Results saved in {log_file}")

if __name__ == "__main__":
    main()
