import os
import sys
import time
import subprocess
import re
import shutil

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    # By default, runs in incremental mode. Add --non-incremental to run non-incremental.
    non_incremental = "--non-incremental" in sys.argv
    incremental = not non_incremental
    graphs_dir = os.path.join(script_dir, "graphs")
    
    # Build original decoder
    print("c Compiling original hcp-decode...")
    subprocess.run(
        ["make", "-C", os.path.join(script_dir, "../refs/ChineseRemainderEncoding"), "hcp-decode"],
        check=True
    )
    
    # Ensure solution_paths directory exists
    solution_paths_dir = os.path.join(script_dir, "solution_paths")
    if os.path.exists(solution_paths_dir):
        shutil.rmtree(solution_paths_dir)
    os.makedirs(solution_paths_dir)
    
    # Find all .edge files in graphs/
    files = [f for f in os.listdir(graphs_dir) if f.endswith(".edge")]
    # Sort files numerically if possible
    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
    
    # CSV Header
    header = "Graph,Total Variables,Total Clauses,Total Runtime (s),Total Solver Time (s),Final Solve Time (s),Status,Verified,Actions,Conflicts,Decisions,Propagations"
    
    log_file = os.path.join(script_dir, "sol.csv")
    with open(log_file, "w") as log:
        log.write(header + "\n")
        
        # Print visual table header on console
        print(f"{'Graph':<15} | {'Variables':<10} | {'Clauses':<10} | {'Total Run (s)':<15} | {'Total Solve (s)':<15} | {'Final Solve (s)':<15} | {'Status':<12} | {'Verified':<10}")
        print("-" * 115)
        
        for file in files:
            graph_path = os.path.join(graphs_dir, file)
            graph_name = os.path.splitext(file)[0]
            
            n_vars = "N/A"
            n_clauses = "N/A"
            solve_time = 0.0
            total_solver_time = "N/A"
            final_solve_time = "N/A"
            actions = "N/A"
            conflicts = "N/A"
            decisions = "N/A"
            propagations = "N/A"
            status = "Unknown"
            verified = "No"
            temp_stdout = os.path.join(script_dir, "temp_run_stdout.sat")
            
            if incremental:
                t_start = time.time()
                try:
                    proc = subprocess.run(
                        [os.path.join(script_dir, "hcp-solver"), graph_path, "--incremental", "--time-limit", "600"],
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                        timeout=610,
                        cwd=script_dir
                    )
                    t_end = time.time()
                    solve_time = t_end - t_start
                    output = proc.stdout + proc.stderr
                    
                    # Parse variable and clause counts and times from stderr
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
                        if "c incremental actions:" in line:
                            try:
                                actions = int(line.split("c incremental actions:")[1].strip())
                            except:
                                pass
                        if "c final solve time:" in line:
                            try:
                                final_solve_time = float(line.split("c final solve time:")[1].strip())
                            except:
                                pass
                        if "c total solver time:" in line:
                            try:
                                total_solver_time = float(line.split("c total solver time:")[1].strip())
                            except:
                                pass
                    
                    # Parse CaDiCaL stats via regex
                    conf_match = re.search(r'conflicts:\s+(\d+)', output)
                    dec_match = re.search(r'decisions:\s+(\d+)', output)
                    prop_match = re.search(r'propagations:\s+(\d+)', output)
                    if conf_match: conflicts = int(conf_match.group(1))
                    if dec_match: decisions = int(dec_match.group(1))
                    if prop_match: propagations = int(prop_match.group(1))
                    
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
                
                if status == "SAT":
                    # Dual Verification
                    try:
                        sol_path = os.path.join(script_dir, "solution.sat")
                        # Create a clean version of the sat file without stats for the naive C decoder
                        clean_sat = os.path.join(script_dir, "temp_clean.sat")
                        with open(sol_path, "r") as infile, open(clean_sat, "w") as outfile:
                            for line in infile:
                                if line.startswith("s ") or line.startswith("v "):
                                    outfile.write(line)
                                    
                        dec_proc = subprocess.run(
                            [os.path.join(script_dir, "hcp-solver"), graph_path, "-d", sol_path],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=script_dir
                        )
                        orig_dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../refs/ChineseRemainderEncoding/hcp-decode"), graph_path, clean_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=script_dir
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
                            verified = "Yes"
                            # Copy solution.path to solution_paths directory
                            source_path = os.path.join(script_dir, "solution.path")
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            if os.path.exists(source_path):
                                shutil.copy(source_path, dest_path)
                        else:
                            verified = "Failed"
                    except Exception as e:
                        verified = "DecErr"
                elif status == "UNSAT":
                    verified = "N/A"
                else:
                    verified = "N/A"
                
                # Clean up solution and clean_sat files
                sol_path = os.path.join(script_dir, "solution.sat")
                clean_sat_path = os.path.join(script_dir, "temp_clean.sat")
                path_file = os.path.join(script_dir, "solution.path")
                for f_tmp in [sol_path, clean_sat_path, path_file]:
                    if os.path.exists(f_tmp):
                        os.remove(f_tmp)
                        
            else:
                temp_cnf = os.path.join(script_dir, "temp_run.cnf")
                test_sat = os.path.join(script_dir, "temp_run.sat")
                
                t_start = time.time()
                # Step 1: Encode
                try:
                    subprocess.run(
                        [os.path.join(script_dir, "hcp-solver"), graph_path, "-c", "420"],
                        stdout=open(temp_cnf, "w"),
                        stderr=subprocess.PIPE,
                        check=True
                    )
                except Exception as e:
                    if os.path.exists(temp_cnf):
                        os.remove(temp_cnf)
                    msg_csv = f"{file},{n_vars},{n_clauses},{solve_time:.2f},{total_solver_time},{final_solve_time},EncodeErr,{verified},{actions},{conflicts},{decisions},{propagations}"
                    log.write(msg_csv + "\n")
                    log.flush()
                    print(f"{file:<15} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'N/A':<15} | {'N/A':<15} | {'EncodeErr':<12} | {'No':<10}")
                    continue
                    
                # Extract variables and clauses from temp_cnf
                try:
                    with open(temp_cnf, "r") as f:
                        first_line = f.readline()
                        if first_line.startswith("p cnf"):
                            parts = first_line.split()
                            n_vars = int(parts[2])
                            n_clauses = int(parts[3])
                except:
                    pass
                    
                # Step 2: Solve with cadical
                actions = 1
                try:
                    with open(temp_stdout, "w") as out_f:
                        proc = subprocess.run(
                            [os.path.join(script_dir, "../refs/cadical/build/cadical"), temp_cnf, "-w", test_sat, "-t", "600"],
                            stdout=out_f,
                            stderr=subprocess.DEVNULL,
                            timeout=610
                        )
                    t_end = time.time()
                    solve_time = t_end - t_start
                    total_solver_time = solve_time
                    final_solve_time = solve_time
                    
                    # Check status and parse stats from temp_stdout line-by-line (prevents large memory footprint)
                    is_sat = False
                    is_unsat = False
                    if os.path.exists(temp_stdout):
                        with open(temp_stdout, "r") as f:
                            for line in f:
                                if "SATISFIABLE" in line:
                                    is_sat = True
                                elif "UNSATISFIABLE" in line:
                                    is_unsat = True
                                conf_match = re.search(r'conflicts:\s+(\d+)', line)
                                dec_match = re.search(r'decisions:\s+(\d+)', line)
                                prop_match = re.search(r'propagations:\s+(\d+)', line)
                                if conf_match: conflicts = int(conf_match.group(1))
                                if dec_match: decisions = int(dec_match.group(1))
                                if prop_match: propagations = int(prop_match.group(1))
                                
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
                if status == "SAT":
                    try:
                        clean_sat = os.path.join(script_dir, "temp_clean.sat")
                        with open(test_sat, "r") as infile, open(clean_sat, "w") as outfile:
                            for line in infile:
                                if line.startswith("s ") or line.startswith("v "):
                                    outfile.write(line)
                                    
                        dec_proc = subprocess.run(
                            [os.path.join(script_dir, "hcp-solver"), graph_path, "-d", test_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=script_dir
                        )
                        orig_dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../refs/ChineseRemainderEncoding/hcp-decode"), graph_path, clean_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=script_dir
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
                            verified = "Yes"
                            # Copy solution.path to solution_paths directory
                            source_path = os.path.join(script_dir, "solution.path")
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            if os.path.exists(source_path):
                                shutil.copy(source_path, dest_path)
                        else:
                            verified = "Failed"
                    except Exception as e:
                        verified = "DecErr"
                elif status == "UNSAT":
                    verified = "N/A"
                else:
                    verified = "N/A"
                    
                # Clean up temp files
                clean_sat_path = os.path.join(script_dir, "temp_clean.sat")
                path_file = os.path.join(script_dir, "solution.path")
                for f_tmp in [temp_cnf, test_sat, clean_sat_path, path_file]:
                    if os.path.exists(f_tmp):
                        os.remove(f_tmp)
                        
            if os.path.exists(temp_stdout):
                os.remove(temp_stdout)
                
            # Write to CSV log
            msg_csv = f"{file},{n_vars},{n_clauses},{solve_time:.2f},{total_solver_time},{final_solve_time},{status},{verified},{actions},{conflicts},{decisions},{propagations}"
            log.write(msg_csv + "\n")
            log.flush()
            
            # Print stats on console
            tot_solve_str = f"{total_solver_time:.2f}" if isinstance(total_solver_time, float) else str(total_solver_time)
            fin_solve_str = f"{final_solve_time:.2f}" if isinstance(final_solve_time, float) else str(final_solve_time)
            print(f"{file:<15} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {tot_solve_str:<15} | {fin_solve_str:<15} | {status:<12} | {verified:<10}")
            
    print(f"\nAll experiments finished. Results saved in CSV format at {log_file}")

if __name__ == "__main__":
    main()
