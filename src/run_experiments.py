import os
import sys
import time
import subprocess
import re

def main():
    incremental = "--incremental" in sys.argv
    graphs_dir = "graphs"
    # Find all .edge files in graphs/
    files = [f for f in os.listdir(graphs_dir) if f.endswith(".edge")]
    # Sort files numerically if possible
    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
    
    print(f"{'Graph':<15} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}")
    print("-" * 85)
    
    log_file = "sol.log"
    with open(log_file, "w") as log:
        log.write(f"{'Graph':<15} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10}\n")
        log.write("-" * 85 + "\n")
        
        for file in files:
            graph_path = os.path.join(graphs_dir, file)
            
            if incremental:
                n_vars = "N/A"
                n_clauses = "N/A"
                t_start = time.time()
                status = "Unknown"
                solve_time = 0.0
                try:
                    proc = subprocess.run(
                        ["./hcp-solver", graph_path, "--incremental", "--time-limit", "600"],
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                        timeout=610
                    )
                    t_end = time.time()
                    solve_time = t_end - t_start
                    output = proc.stdout + proc.stderr
                    
                    # Parse variable and clause counts from stderr
                    for line in proc.stderr.split("\n"):
                        if "c total variables:" in line:
                            try:
                                n_vars = int(line.split("c total variables:")[1].strip())
                            except:
                                pass
                        if "c total clauses:" in line:
                            try:
                                n_clauses = int(line.split("c total clauses:")[1].strip())
                            except:
                                pass
                    
                    if "c HAMILTONIAN found" in output:
                        status = "SAT"
                    elif "c UNSAT" in output:
                        status = "UNSAT"
                    elif "c TIMEOUT" in output:
                        status = "Timeout"
                    else:
                        status = "Unknown"
                except subprocess.TimeoutExpired:
                    solve_time = 600.0
                    status = "Timeout"
                except Exception as e:
                    status = "SolveErr"
                
                verified = "No"
                if status == "SAT":
                    try:
                        dec_proc = subprocess.run(
                            ["./hcp-solver", graph_path, "-d", "solution.sat"],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            check=True
                        )
                        if "VERIFIED" in dec_proc.stdout:
                            verified = "Yes"
                        else:
                            verified = "Failed"
                    except Exception as e:
                        verified = "DecErr"
                elif status == "UNSAT":
                    verified = "N/A"
                else:
                    verified = "N/A"
                
                # Clean up solution file
                if os.path.exists("solution.sat"):
                    os.remove("solution.sat")
            else:
                temp_cnf = "temp_run.cnf"
                test_sat = "temp_run.sat"
                
                # Step 1: Encode
                try:
                    subprocess.run(
                        ["./hcp-solver", graph_path],
                        stdout=open(temp_cnf, "w"),
                        stderr=subprocess.PIPE,
                        check=True
                    )
                except Exception as e:
                    msg = f"{file:<15} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'EncodeErr':<12} | {'No':<10}"
                    print(msg)
                    log.write(msg + "\n")
                    log.flush()
                    continue
                    
                # Extract variables and clauses from temp_cnf
                n_vars = 0
                n_clauses = 0
                try:
                    with open(temp_cnf, "r") as f:
                        first_line = f.readline()
                        if first_line.startswith("p cnf"):
                            parts = first_line.split()
                            n_vars = int(parts[2])
                            n_clauses = int(parts[3])
                except Exception as e:
                    pass
                    
                # Step 2: Solve with cadical
                # Timeout is 600s
                t_start = time.time()
                status = "Unknown"
                solve_time = 0.0
                
                try:
                    proc = subprocess.run(
                        ["../refs/cadical/build/cadical", temp_cnf, "-t", "600"],
                        stdout=open(test_sat, "w"),
                        stderr=subprocess.PIPE,
                        timeout=610
                    )
                    t_end = time.time()
                    solve_time = t_end - t_start
                    
                    # Check status
                    is_sat = False
                    is_unsat = False
                    with open(test_sat, "r") as f:
                        for line in f:
                            if "SATISFIABLE" in line:
                                is_sat = True
                                break
                            elif "UNSATISFIABLE" in line:
                                is_unsat = True
                                break
                                
                    if is_sat or proc.returncode == 10:
                        status = "SAT"
                    elif is_unsat or proc.returncode == 20:
                        status = "UNSAT"
                    else:
                        status = "Timeout"
                except subprocess.TimeoutExpired:
                    solve_time = 600.0
                    status = "Timeout"
                except Exception as e:
                    status = "SolveErr"
                    
                # Step 3: Decode/Verify if SAT
                verified = "No"
                if status == "SAT":
                    try:
                        dec_proc = subprocess.run(
                            ["./hcp-solver", graph_path, "-d", test_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            check=True
                        )
                        if "VERIFIED" in dec_proc.stdout:
                            verified = "Yes"
                        else:
                            verified = "Failed"
                    except Exception as e:
                        verified = "DecErr"
                elif status == "UNSAT":
                    verified = "N/A"
                else:
                    verified = "N/A"
                    
                # Clean up temp files
                for f_tmp in [temp_cnf, test_sat]:
                    if os.path.exists(f_tmp):
                        os.remove(f_tmp)
                    
            msg = f"{file:<15} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {status:<12} | {verified:<10}"
            print(msg)
            log.write(msg + "\n")
            log.flush()
            
    print(f"\nAll experiments finished. Results saved in {log_file}")

if __name__ == "__main__":
    main()
