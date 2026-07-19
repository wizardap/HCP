import os
import sys
import time
import subprocess
import re
import shutil

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    non_incremental = "--non-incremental" in sys.argv
    incremental = not non_incremental
    
    # Parse time-limit argument (default: 600)
    time_limit = 600
    if "-t" in sys.argv:
        try:
            time_limit = int(sys.argv[sys.argv.index("-t") + 1])
        except (IndexError, ValueError):
            pass
    elif "--time-limit" in sys.argv:
        try:
            time_limit = int(sys.argv[sys.argv.index("--time-limit") + 1])
        except (IndexError, ValueError):
            pass



    # Resolve to root-level graphs/
    graphs_dir = os.path.join(script_dir, "../graphs")
    
    # Build original decoder
    print("c Compiling original hcp-decode...")
    subprocess.run(
        ["make", "-C", os.path.join(script_dir, "../refs/ChineseRemainderEncoding"), "hcp-decode"],
        check=True
    )
    
    # Ensure root-level solution_paths directory exists
    solution_paths_dir = os.path.join(script_dir, "../solution_paths")
    if os.path.exists(solution_paths_dir):
        shutil.rmtree(solution_paths_dir)
    os.makedirs(solution_paths_dir)
    
    # Find all .edge files in graphs/ recursively
    files = []
    for root, _, filenames in os.walk(graphs_dir):
        for f in filenames:
            if f.endswith(".edge"):
                rel_path = os.path.relpath(os.path.join(root, f), graphs_dir)
                files.append(rel_path)
                
    # Filter by graph substring if requested
    graph_filter = None
    if "-g" in sys.argv:
        try:
            graph_filter = sys.argv[sys.argv.index("-g") + 1]
        except IndexError:
            pass
    elif "--graph" in sys.argv:
        try:
            graph_filter = sys.argv[sys.argv.index("--graph") + 1]
        except IndexError:
            pass
            
    if graph_filter:
        if graph_filter == "fhcppp":
            fhcppp_names = {
                "graph48.edge", "graph162.edge", "graph171.edge", "graph197.edge",
                "graph223.edge", "graph237.edge", "graph249.edge", "graph252.edge",
                "graph254.edge", "graph255.edge", "graph424.edge", "graph446.edge",
                "graph470.edge", "graph491.edge", "graph506.edge", "graph522.edge",
                "graph526.edge", "graph529.edge"
            }
            files = [f for f in files if os.path.basename(f) in fhcppp_names and os.path.dirname(f) == ""]
        else:
            files = [f for f in files if graph_filter in f]
    
    # Sort files by subdirectory, numerically, then by filename string
    def get_sort_key(filename):
        subdir = os.path.dirname(filename)
        base = os.path.basename(filename)
        match = re.search(r'\d+', base)
        num = int(match.group()) if match else float('inf')
        return (subdir, num, filename)
    files.sort(key=get_sort_key)
    
    # CSV Header
    header = "Graph,Total Variables,Total Clauses,Total Runtime (s),Total Solver Time (s),Final Solve Time (s),Status,Verified,Actions,Conflicts,Decisions,Propagations"
    
    # Resolve to root-level sol.csv
    log_file = os.path.join(script_dir, "../sol.csv")
    with open(log_file, "w") as log:
        log.write(header + "\n")
        
        # Print visual table header on console
        print(f"{'Graph':<35} | {'Variables':<10} | {'Clauses':<10} | {'Total Run (s)':<15} | {'Total Solve (s)':<15} | {'Final Solve (s)':<15} | {'Status':<12} | {'Verified':<10}")
        print("-" * 135)
        
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
            temp_stdout = os.path.join(script_dir, "../src/temp_run_stdout.sat")
            
            if incremental:
                t_start = time.time()
                try:
                    proc = subprocess.run(
                        [os.path.join(script_dir, "../src/hcp-solver"), graph_path, "--incremental", "--cycle", "auto", "--time-limit", str(time_limit)],
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                        timeout=time_limit + 10,
                        cwd=os.path.join(script_dir, "../src")
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
                    solve_time = float(time_limit)
                    status = "Timeout"
                except Exception as e:
                    status = "SolveErr"
                
                if status == "SAT":
                    # Dual Verification
                    try:
                        sol_path = os.path.join(script_dir, "../src/solution.sat")
                        # Create a clean version of the sat file without stats for the naive C decoder
                        clean_sat = os.path.join(script_dir, "../src/temp_clean.sat")
                        with open(sol_path, "r") as infile, open(clean_sat, "w") as outfile:
                            for line in infile:
                                if line.startswith("s ") or line.startswith("v "):
                                    outfile.write(line)
                                    
                        # Create a clean version of the graph file without duplicate edges
                        clean_graph_path = os.path.join(script_dir, "../src/temp_clean_graph.edge")
                        edges_seen = set()
                        unique_edges = []
                        n_nodes_found = 0
                        
                        with open(graph_path, "r") as gf:
                            for line in gf:
                                stripped = line.strip()
                                if not stripped or stripped.startswith("c") or stripped.startswith("C"):
                                    continue
                                if stripped.startswith("p edge"):
                                    parts = stripped.split()
                                    n_nodes_found = int(parts[2])
                                    continue
                                parts = stripped.split()
                                if len(parts) < 2:
                                    continue
                                first = parts[0]
                                if first == "e" or first == "E":
                                    u, v = int(parts[1]), int(parts[2])
                                else:
                                    try:
                                        u = int(first)
                                        v = int(parts[1])
                                    except ValueError:
                                        continue
                                unique_edges.append((u, v))
                                        
                        with open(clean_graph_path, "w") as cgf:
                            cgf.write(f"p edge {n_nodes_found} {len(unique_edges)}\n")
                            for u, v in unique_edges:
                                cgf.write(f"e {u} {v}\n")

                        dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../src/hcp-solver"), clean_graph_path, "-d", sol_path],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=os.path.join(script_dir, "../src")
                        )
                        orig_dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../refs/ChineseRemainderEncoding/hcp-decode"), clean_graph_path, clean_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=os.path.join(script_dir, "../src")
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
                            verified = "Yes"
                            # Copy solution.path to solution_paths directory
                            source_path = os.path.join(script_dir, "../src/solution.path")
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
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
                
                # Clean up solution, clean_sat, clean_graph, and path files
                sol_path = os.path.join(script_dir, "../src/solution.sat")
                clean_sat_path = os.path.join(script_dir, "../src/temp_clean.sat")
                clean_graph_path = os.path.join(script_dir, "../src/temp_clean_graph.edge")
                path_file = os.path.join(script_dir, "../src/solution.path")
                for f_tmp in [sol_path, clean_sat_path, clean_graph_path, path_file]:
                    if os.path.exists(f_tmp):
                        os.remove(f_tmp)
                        
            else:
                temp_cnf = os.path.join(script_dir, "../src/temp_run.cnf")
                test_sat = os.path.join(script_dir, "../src/temp_run.sat")
                
                t_start = time.time()
                # Step 1: Encode
                try:
                    with open(temp_cnf, "w") as out_f:
                        subprocess.run(
                            [os.path.join(script_dir, "../src/hcp-solver"), graph_path, "-c", "420"],
                            stdout=out_f,
                            stderr=subprocess.PIPE,
                            check=True
                        )
                except Exception as e:
                    if os.path.exists(temp_cnf):
                        os.remove(temp_cnf)
                    msg_csv = f"{file},{n_vars},{n_clauses},{solve_time:.2f},{total_solver_time},{final_solve_time},EncodeErr,{verified},{actions},{conflicts},{decisions},{propagations}"
                    log.write(msg_csv + "\n")
                    log.flush()
                    print(f"{file:<35} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'N/A':<15} | {'N/A':<15} | {'EncodeErr':<12} | {'No':<10}")
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
                            [os.path.join(script_dir, "../refs/cadical/build/cadical"), temp_cnf, "-w", test_sat, "-t", str(time_limit)],
                            stdout=out_f,
                            stderr=subprocess.DEVNULL,
                            timeout=time_limit + 10
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
                    solve_time = float(time_limit)
                    status = "Timeout"
                except Exception as e:
                    status = "SolveErr"
                    
                # Step 3: Decode/Verify if SAT
                if status == "SAT":
                    try:
                        clean_sat = os.path.join(script_dir, "../src/temp_clean.sat")
                        with open(test_sat, "r") as infile, open(clean_sat, "w") as outfile:
                            for line in infile:
                                if line.startswith("s ") or line.startswith("v "):
                                    outfile.write(line)
                                    
                        dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../src/hcp-solver"), graph_path, "-d", test_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=os.path.join(script_dir, "../src")
                        )
                        orig_dec_proc = subprocess.run(
                            [os.path.join(script_dir, "../refs/ChineseRemainderEncoding/hcp-decode"), graph_path, clean_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            cwd=os.path.join(script_dir, "../src")
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
                            verified = "Yes"
                            # Copy solution.path to solution_paths directory
                            source_path = os.path.join(script_dir, "../src/solution.path")
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
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
                clean_sat_path = os.path.join(script_dir, "../src/temp_clean.sat")
                path_file = os.path.join(script_dir, "../src/solution.path")
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
            print(f"{file:<35} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {tot_solve_str:<15} | {fin_solve_str:<15} | {status:<12} | {verified:<10}")
            
    print(f"\nAll experiments finished. Results saved in CSV format at {log_file}")

if __name__ == "__main__":
    main()
