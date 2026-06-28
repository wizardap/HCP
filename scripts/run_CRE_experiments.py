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
    files = [f for f in os.listdir(GRAPHS_DIR) if f.endswith(".edge")]

    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)

    if args.graph:
        files = [f for f in files if args.graph in f]
        if not files:
            print(f"No graphs matching '{args.graph}'")
            return

    print(f"{'Graph':<15} | {'Vertices':<9} | {'Cycle':<7} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}")
    print("-" * 100)

    log_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "CRE_sol.log")
    with open(log_file, "w") as log:
        log.write(f"{'Graph':<15} | {'Vertices':<9} | {'Cycle':<7} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}\n")
        log.write("-" * 100 + "\n")

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

            temp_cnf = "temp_cre.cnf"
            temp_sat = "temp_cre.sat"

            # Step 1: Encode with CRE
            n_vars = 0
            n_clauses = 0
            try:
                with open(temp_cnf, "w") as f:
                    proc = subprocess.run(
                        cmd,
                        stdout=f,
                        stderr=subprocess.PIPE,
                        check=True
                    )

                # Parse header
                with open(temp_cnf, "r") as f:
                    first_line = f.readline()
                    if first_line.startswith("p cnf"):
                        parts = first_line.split()
                        n_vars = int(parts[2])
                        n_clauses = int(parts[3])
            except Exception as e:
                cycle_str = str(args.cycle) if args.cycle is not None else "auto"
                msg = f"{file:<15} | {nNode:<9} | {cycle_str:<7} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'EncodeErr':<12} | {'No':<10}"
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
            msg = f"{file:<15} | {nNode:<9} | {cycle_str:<7} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {status:<12} | {verified:<10}"
            print(msg)
            log.write(msg + "\n")
            log.flush()

    print(f"\nAll CRE experiments finished. Results saved in {log_file}")

if __name__ == "__main__":
    main()
