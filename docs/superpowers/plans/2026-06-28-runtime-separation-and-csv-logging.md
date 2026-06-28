# Runtime Separation, CSV Logging, and Path Gathering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modify the HCP solver pipeline to separate total runtime from solver execution times, obtain variable count from the CaDiCaL C API, log results in CSV format, gather solution path files, and make incremental solving the default path.

**Architecture:**
1. Expose `ccadical_vars(solver)` for programmatically querying variables via C API.
2. Measure steady clock durations for all `ccadical_solve()` calls within `IncrementalSolver` to track latest solve time and accumulated solve time.
3. Log solver metrics to `stderr` from `Solver.cpp` on exit.
4. Update `run_experiments.py` to write to `src/sol.csv`, make incremental the default execution, copy path files to `src/solution_paths/`, and run CaDiCaL with `-w` to prevent witness stats interleaving.

**Tech Stack:** C++17, Python 3, gcc, CaDiCaL SAT solver API

## Global Constraints
- Do not modify any files in the `HCP/refs/` directory (specifically `refs/cadical/`).
- Standard DIMACS and CaDiCaL statistics output formats must be preserved.
- All temporary CNF and SAT files must be cleaned up properly after verification.
- Output path sequence must be space-separated in `solution.path`.

---

### Task 1: Expose API variable count and Timing Metrics in IncrementalSolver

**Files:**
- Modify: `src/IncrementalSolver.hpp`
- Modify: `src/IncrementalSolver.cpp`
- Modify: `src/test_incremental_solver.cpp`

**Interfaces:**
- Consumes: CaDiCaL C API `ccadical_vars(solver)` from `refs/cadical/src/ccadical.h`
- Produces:
  - `int IncrementalSolver::getNumVars() const` (updated to query API)
  - `double IncrementalSolver::getFinalSolveTime() const`
  - `double IncrementalSolver::getTotalSolverTime() const`

- [ ] **Step 1: Declare member variables and getters in `src/IncrementalSolver.hpp`**

  Modify `src/IncrementalSolver.hpp` around line 54:
  ```cpp
      // Returns the maximum variable index added or solved.
      int getNumVars() const;

      // Returns the total number of clauses added.
      int64_t getNumClauses() const;

      // NEW: Get timing statistics in seconds
      double getFinalSolveTime() const;
      double getTotalSolverTime() const;

      // NEW: Print CaDiCaL statistics
      void printStatistics() const;
  ```

  And add the private members around line 71:
  ```cpp
  private:
      CCaDiCaL* solver = nullptr;
      int max_var = 0;
      int64_t numClauses = 0;
      int64_t timeLimitMs = 0;
      std::chrono::steady_clock::time_point startTime;
      SolverState state = SolverState::UNSOLVED;

      // NEW: Timings tracking in seconds
      double finalSolveTime = 0.0;
      double totalSolverTime = 0.0;

      bool checkTimeout() const;
  ```

- [ ] **Step 2: Update implementation in `src/IncrementalSolver.cpp`**

  Update `IncrementalSolver::solve()` around line 82:
  ```cpp
  IncrementalSolver::Result IncrementalSolver::solve() {
      startTime = std::chrono::steady_clock::now();
      if (timeLimitMs > 0) {
          ccadical_set_terminate(solver, this, ccadical_terminate_callback);
      } else {
          ccadical_set_terminate(solver, nullptr, nullptr);
      }

      auto solveStart = std::chrono::steady_clock::now();
      int res = ccadical_solve(solver);
      auto solveEnd = std::chrono::steady_clock::now();

      // Clean up callback after solving
      ccadical_set_terminate(solver, nullptr, nullptr);

      double duration = std::chrono::duration<double>(solveEnd - solveStart).count();
      finalSolveTime = duration;
      totalSolverTime += duration;

      if (res == 10) {
          state = SolverState::SAT;
          return Result::SAT;
      } else if (res == 20) {
          state = SolverState::UNSAT;
          return Result::UNSAT;
      } else {
          state = SolverState::TIMEOUT;
          return Result::TIMEOUT;
      }
  }
  ```

  And update `getNumVars()` and getters around line 139:
  ```cpp
  int IncrementalSolver::getNumVars() const {
      if (solver) {
          return ccadical_vars(solver);
      }
      return max_var;
  }

  int64_t IncrementalSolver::getNumClauses() const {
      return numClauses;
  }

  double IncrementalSolver::getFinalSolveTime() const {
      return finalSolveTime;
  }

  double IncrementalSolver::getTotalSolverTime() const {
      return totalSolverTime;
  }
  ```

- [ ] **Step 3: Add test assertions in `src/test_incremental_solver.cpp`**

  Modify `src/test_incremental_solver.cpp` in `testIncrementalSolverBasic` (around line 40):
  ```cpp
      assert(solver.getModelValue(1) == 1);
      assert(solver.getModelValue(2) == -1);
      
      std::cout << "Testing printStatistics():\n";
      solver.printStatistics();

      // Verify solve times
      assert(solver.getFinalSolveTime() >= 0.0);
      assert(solver.getTotalSolverTime() >= 0.0);
      std::cout << "Solve times verified: final=" << solver.getFinalSolveTime() 
                << "s, total=" << solver.getTotalSolverTime() << "s\n";
  ```

- [ ] **Step 4: Build and run test_incremental_solver**

  Run:
  ```bash
  make -C src clean && make -C src test_incremental_solver && src/test_incremental_solver
  ```
  Expected:
  - Compiles successfully.
  - Test runs, prints solve times, and passes all assertions.

- [ ] **Step 5: Commit changes**

  Run:
  ```bash
  git add src/IncrementalSolver.hpp src/IncrementalSolver.cpp src/test_incremental_solver.cpp
  git commit -m "feat: track API variables and solve timing metrics in IncrementalSolver"
  ```

---

### Task 2: Print Timing and API Variable Metrics on Exit in Solver.cpp

**Files:**
- Modify: `src/Solver.cpp`

**Interfaces:**
- Consumes: `isolver.getNumVars()`, `isolver.getNumClauses()`, `isolver.getFinalSolveTime()`, and `isolver.getTotalSolverTime()`
- Produces: `stderr` stats blocks on loop termination

- [ ] **Step 1: Update Solver.cpp runIncremental outputs**

  Modify `src/Solver.cpp` inside `Solver::runIncremental` on exit conditions (around lines 88-135):
  ```cpp
      int actions = 0;
      while (true) {
          actions++;
          auto result = isolver.solve();
          if (result == IncrementalSolver::Result::UNSAT) {
              std::cerr << "c UNSAT\n";
              std::cerr << "c incremental actions: " << actions << "\n";
              std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
              std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
              std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
              std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
              isolver.printStatistics();
              return false;
          }
          if (result == IncrementalSolver::Result::TIMEOUT) {
              std::cerr << "c TIMEOUT\n";
              std::cerr << "c incremental actions: " << actions << "\n";
              std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
              std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
              std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
              std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
              isolver.printStatistics();
              return false;
          }
          if (result == IncrementalSolver::Result::SAT) {
              auto model = isolver.getModel();
              auto components = SubtourDetector::detect(model, g);
              if (components.empty()) {
                  std::cerr << "c HAMILTONIAN found\n";
                  std::string solFile = "solution.sat";
                  std::ofstream solOut(solFile);
                  if (!solOut.is_open() || solOut.fail()) {
                      std::cerr << "c Error: Could not write solution to " << solFile << "\n";
                      return false;
                  }
                  solOut << "s SATISFIABLE\nv ";
                  for (int var = 1; var <= isolver.getNumVars(); ++var) {
                      int val = isolver.getModelValue(var);
                      if (val > 0) {
                          solOut << var << " ";
                      } else if (val < 0) {
                          solOut << -var << " ";
                      }
                  }
                  solOut << "0\n";
                  if (solOut.fail()) {
                      std::cerr << "c Error: Failed while writing solution to " << solFile << "\n";
                      solOut.close();
                      return false;
                  }
                  solOut.close();
                  
                  std::cerr << "c incremental actions: " << actions << "\n";
                  std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                  std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                  std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                  std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                  isolver.printStatistics();
                  return true;
              } else {
                  SecEncoder secEncoder(g);
                  auto secClauses = secEncoder.encodeSecs(components);
                  for (const auto& clause : secClauses) {
                      isolver.addClause(clause);
                  }
                  std::cerr << "c Iteration: found " << components.size() 
                            << " components, added " << secClauses.size() << " SEC clauses\n";
              }
          }
      }
  ```

- [ ] **Step 2: Compile the solver suite**

  Run:
  ```bash
  make -C src clean && make -C src
  ```
  Expected:
  - Compiles successfully with no warnings.

- [ ] **Step 3: Commit changes**

  Run:
  ```bash
  git add src/Solver.cpp
  git commit -m "feat: output final solve time and total solver time on exit in Solver.cpp"
  ```

---

### Task 3: Update run_experiments.py for CSV Output, Directory Independence, CaDiCaL -w and Path Gathering

**Files:**
- Modify: `src/run_experiments.py`

**Interfaces:**
- Consumes: Stderr/stdout from `hcp-solver` and writes to `src/sol.csv`
- Produces: `src/sol.csv` file, gathers solution paths to `src/solution_paths/`

- [ ] **Step 1: Rewrite `src/run_experiments.py`**

  Replace the complete contents of `src/run_experiments.py` with:
  ```python
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
                  for f_tmp in [sol_path, clean_sat_path]:
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
                  for f_tmp in [temp_cnf, test_sat, clean_sat_path]:
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
  ```

- [ ] **Step 2: Commit runner modifications**

  Run:
  ```bash
  git add src/run_experiments.py
  git commit -m "feat: support CSV output, path gathering, and directory independence in run_experiments.py"
  ```
