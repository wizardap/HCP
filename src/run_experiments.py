import os
import sys
import time
import subprocess
import re

def main():
    incremental = "--incremental" in sys.argv
    graphs_dir = "graphs"
    
    # Build original decoder
    print("c Compiling original hcp-decode...")
    subprocess.run(
        ["make", "-C", "../refs/ChineseRemainderEncoding", "hcp-decode"],
        check=True
    )
    
    # Find all .edge files in graphs/
    files = [f for f in os.listdir(graphs_dir) if f.endswith(".edge")]
    # Sort files numerically if possible
    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
    
    # Header
    header = f"{'Graph':<15} | {'Variables':<10} | {'Clauses':<10} | {'Solve Time (s)':<15} | {'Status':<12} | {'Verified':<10} | {'Actions':<8} | {'Conflicts':<10} | {'Decisions':<10} | {'Propagations':<12}"
    separator = "-" * 135
    print(header)
    print(separator)
    
    log_file = "sol.log"
    with open(log_file, "w") as log:
        log.write(header + "\n")
        log.write(separator + "\n")
        
        for file in files:
            graph_path = os.path.join(graphs_dir, file)
            
            n_vars = "N/A"
            n_clauses = "N/A"
            solve_time = 0.0
            actions = "N/A"
            conflicts = "N/A"
            decisions = "N/A"
            propagations = "N/A"
            status = "Unknown"
            verified = "No"
            
            if incremental:
                t_start = time.time()
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
                        if "c incremental actions:" in line:
                            try:
                                actions = int(line.split("c incremental actions:")[1].strip())
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
                        # 1. C++ Decoder
                        dec_proc = subprocess.run(
                            ["./hcp-solver", graph_path, "-d", "solution.sat"],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            check=True
                        )
                        # 2. Original C Decoder
                        orig_dec_proc = subprocess.run(
                            ["../refs/ChineseRemainderEncoding/hcp-decode", graph_path, "solution.sat"],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            check=True
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
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
                
                t_start = time.time()
                # Step 1: Encode
                try:
                    with open(temp_cnf, "w") as out_f:
                        subprocess.run(
                            ["./hcp-solver", graph_path, "-c", "420"],
                            stdout=out_f,
                            stderr=subprocess.PIPE,
                            check=True
                        )
                except Exception as e:
                    if os.path.exists(temp_cnf):
                        os.remove(temp_cnf)
                    msg = f"{file:<15} | {'Error':<10} | {'Error':<10} | {'0.00':<15} | {'EncodeErr':<12} | {'No':<10} | {'N/A':<8} | {'N/A':<10} | {'N/A':<10} | {'N/A':<12}"
                    print(msg)
                    log.write(msg + "\n")
                    log.flush()
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
                    with open(test_sat, "w") as out_f:
                        proc = subprocess.run(
                            ["../refs/cadical/build/cadical", temp_cnf, "-t", "600"],
                            stdout=out_f,
                            stderr=subprocess.PIPE,
                            timeout=610
                        )
                    t_end = time.time()
                    solve_time = t_end - t_start
                    
                    # Check status
                    is_sat = False
                    is_unsat = False
                    with open(test_sat, "r") as f:
                        sat_content = f.read()
                        if "SATISFIABLE" in sat_content:
                            is_sat = True
                        elif "UNSATISFIABLE" in sat_content:
                            is_unsat = True
                            
                        # Parse stats from temp_run.sat
                        conf_match = re.search(r'conflicts:\s+(\d+)', sat_content)
                        dec_match = re.search(r'decisions:\s+(\d+)', sat_content)
                        prop_match = re.search(r'propagations:\s+(\d+)', sat_content)
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
                        # Create a clean version of the sat file without stats for the naive C decoder
                        clean_sat = "temp_clean.sat"
                        with open(test_sat, "r") as infile, open(clean_sat, "w") as outfile:
                            for line in infile:
                                if line.startswith("s ") or line.startswith("v "):
                                    outfile.write(line)

                        dec_proc = subprocess.run(
                            ["./hcp-solver", graph_path, "-d", test_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            check=True
                        )
                        orig_dec_proc = subprocess.run(
                            ["../refs/ChineseRemainderEncoding/hcp-decode", graph_path, clean_sat],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True,
                            timeout=10,
                            check=True
                        )
                        if "VERIFIED" in dec_proc.stdout and "VERIFIED" in orig_dec_proc.stdout:
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
                for f_tmp in [temp_cnf, test_sat, "temp_clean.sat"]:
                    if os.path.exists(f_tmp):
                        os.remove(f_tmp)
                    
            msg = f"{file:<15} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {status:<12} | {verified:<10} | {actions:<8} | {conflicts:<10} | {decisions:<10} | {propagations:<12}"
            print(msg)
            log.write(msg + "\n")
            log.flush()
            
    print(f"\nAll experiments finished. Results saved in {log_file}")

if __name__ == "__main__":
    main()
