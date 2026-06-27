# Incremental Metrics and Dual Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement CaDiCaL metrics extraction, incremental action counting, total solving time measurement, and dual verification (using the original C hcp-decode and C++ node path output) for the Hamiltonian Cycle SAT solver.

**Architecture:** 
1. `IncrementalSolver` wraps CaDiCaL's `ccadical_print_statistics()` to write statistics to `stdout`.
2. `Solver::runIncremental` prints final counts and triggers `isolver.printStatistics()`.
3. `HcpDecoder` collects cycle path nodes during verification and writes them to `solution.path`.
4. `run_experiments.py` compiles the original C `hcp-decode`, measures total solving time from encoding start, parses statistics via regexes from outputs, and verifies with both decoders.

**Tech Stack:** C++17, Python 3, gcc, CaDiCaL SAT solver API

## Global Constraints
- Do not modify any files in the `HCP/refs/` directory (specifically `refs/cadical/`).
- Standard DIMACS and CaDiCaL statistics output formats must be preserved.
- All temporary CNF and SAT files must be cleaned up properly after verification.
- Output path sequence must be space-separated in `solution.path`.

---

### Task 1: Expose printStatistics in IncrementalSolver

**Files:**
- Modify: `src/IncrementalSolver.hpp`
- Modify: `src/IncrementalSolver.cpp`
- Modify: `src/test_incremental_solver.cpp`

**Interfaces:**
- Consumes: CaDiCaL C API `ccadical_print_statistics()` from `refs/cadical/src/ccadical.h`
- Produces: `void IncrementalSolver::printStatistics() const`

- [ ] **Step 1: Declare `printStatistics` in `src/IncrementalSolver.hpp`**

  Modify `src/IncrementalSolver.hpp` around line 59:
  ```cpp
      // Returns the total number of clauses added.
      int64_t getNumClauses() const;

      // NEW: Print CaDiCaL statistics
      void printStatistics() const;

      // Sets the execution time limit in milliseconds.
  ```

- [ ] **Step 2: Implement `printStatistics` in `src/IncrementalSolver.cpp`**

  Modify `src/IncrementalSolver.cpp` around line 268:
  ```cpp
  int64_t IncrementalSolver::getNumClauses() const {
      return numClauses;
  }

  void IncrementalSolver::printStatistics() const {
      ccadical_print_statistics(solver);
  }

  void IncrementalSolver::setTimeLimit(int64_t ms) {
  ```

- [ ] **Step 3: Add compile test in `src/test_incremental_solver.cpp`**

  Modify `src/test_incremental_solver.cpp` at the end of the `test_basic_sat` function (around line 34):
  ```cpp
      assert(solver.getModelValue(1) == 1);
      assert(solver.getModelValue(2) == -1);
      
      std::cout << "Testing printStatistics():\n";
      solver.printStatistics();
  ```

- [ ] **Step 4: Build and run incremental solver tests**

  Run:
  ```bash
  make -C src clean && make -C src test_incremental_solver && src/test_incremental_solver
  ```
  Expected:
  - Compiles successfully.
  - Runs and outputs CaDiCaL statistics block containing `conflicts:`, `decisions:`, etc.
  - Tests pass with no failures.

- [ ] **Step 5: Commit changes**

  Run:
  ```bash
  git add src/IncrementalSolver.hpp src/IncrementalSolver.cpp src/test_incremental_solver.cpp
  git commit -m "feat: expose CaDiCaL printStatistics in IncrementalSolver"
  ```

---

### Task 2: Capture Cycle Node Sequence in HcpDecoder

**Files:**
- Modify: `src/HcpDecoder.hpp`

**Interfaces:**
- Consumes: `std::vector<int> nextNode` reconstructed from solution file
- Produces: `solution.path` file containing space-separated node IDs of the verified cycle

- [ ] **Step 1: Add cycle path capturing and file output logic to `src/HcpDecoder.hpp`**

  Modify `src/HcpDecoder.hpp` inside `decode()` (lines 145-160):
  ```cpp
          visited.assign(nNode + 1, 0);

          std::vector<int> path;
          int a = 1;
          for (int i = 1; i <= nNode + 1; i++) {
              path.push_back(a);
              if (visited[a]) {
                  if ((i - visited[a]) == nNode) {
                      std::cout << "c VERIFIED HCP of size " << nNode << "\n";
                      
                      // Write cycle path to solution.path
                      std::ofstream pathOut("solution.path");
                      if (pathOut.is_open()) {
                          for (size_t k = 0; k < path.size(); ++k) {
                              pathOut << path[k] << (k == path.size() - 1 ? "" : " ");
                          }
                          pathOut << "\n";
                          pathOut.close();
                      }
                  } else {
                      std::cout << "c ERROR: cycle of size " << (i - visited[a]) << " out of " << nNode << "\n";
                  }
                  break;
              }
              visited[a] = i;
              a = nextNode[a];
          }
  ```

- [ ] **Step 2: Add header `#include <fstream>` if not present in `src/HcpDecoder.hpp`**

  Ensure `<fstream>` is included at the top of `src/HcpDecoder.hpp`.

- [ ] **Step 3: Compile the solver suite**

  Run:
  ```bash
  make -C src
  ```
  Expected:
  - Compiles successfully.

- [ ] **Step 4: Commit changes**

  Run:
  ```bash
  git add src/HcpDecoder.hpp
  git commit -m "feat: generate solution.path file containing cycle sequence in HcpDecoder"
  ```

---

### Task 3: Track Incremental Actions and Output Statistics in Solver

**Files:**
- Modify: `src/Solver.cpp`

**Interfaces:**
- Consumes: `isolver.printStatistics()` and variable/clause getters from `IncrementalSolver`
- Produces: Formatted `stderr` metrics block on loop termination

- [ ] **Step 1: Add actions counter and termination print block in `src/Solver.cpp`**

  Modify `src/Solver.cpp` inside `Solver::runIncremental` (lines 86-134):
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
              isolver.printStatistics();
              return false;
          }
          if (result == IncrementalSolver::Result::TIMEOUT) {
              std::cerr << "c TIMEOUT\n";
              std::cerr << "c incremental actions: " << actions << "\n";
              std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
              std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
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
                  solOut << "SATISFIABLE\nv ";
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

- [ ] **Step 2: Build solver and check compilation**

  Run:
  ```bash
  make -C src clean && make -C src
  ```
  Expected:
  - Compilation completes successfully with no errors.

- [ ] **Step 3: Commit changes**

  Run:
  ```bash
  git add src/Solver.cpp
  git commit -m "feat: output incremental actions, variables, clauses, and CaDiCaL statistics in Solver"
  ```

---

### Task 4: Integrate Dual Verification and Metrics Parsing in run_experiments.py

**Files:**
- Modify: `src/run_experiments.py`

**Interfaces:**
- Consumes: Stderr outputs from `./hcp-solver`, `temp_run.sat` for non-incremental mode, and compiled `refs/ChineseRemainderEncoding/hcp-decode`
- Produces: Updated `sol.log` table and dual-verified SAT solutions

- [ ] **Step 1: Modify `src/run_experiments.py` to compile the original decoder and parse metrics**

  Rewrite `src/run_experiments.py` (lines 1-206):
  ```python
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
                              check=True
                          )
                          # 2. Original C Decoder
                          orig_dec_proc = subprocess.run(
                              ["../refs/ChineseRemainderEncoding/hcp-decode", graph_path, "solution.sat"],
                              stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE,
                              text=True,
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
                      subprocess.run(
                          ["./hcp-solver", graph_path, "-c", "420"],
                          stdout=open(temp_cnf, "w"),
                          stderr=subprocess.PIPE,
                          check=True
                      )
                  except Exception as e:
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
                          dec_proc = subprocess.run(
                              ["./hcp-solver", graph_path, "-d", test_sat],
                              stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE,
                              text=True,
                              check=True
                          )
                          orig_dec_proc = subprocess.run(
                              ["../refs/ChineseRemainderEncoding/hcp-decode", graph_path, test_sat],
                              stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE,
                              text=True,
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
                  for f_tmp in [temp_cnf, test_sat]:
                      if os.path.exists(f_tmp):
                          os.remove(f_tmp)
                      
              msg = f"{file:<15} | {n_vars:<10} | {n_clauses:<10} | {solve_time:<15.2f} | {status:<12} | {verified:<10} | {actions:<8} | {conflicts:<10} | {decisions:<10} | {propagations:<12}"
              print(msg)
              log.write(msg + "\n")
              log.flush()
              
      print(f"\nAll experiments finished. Results saved in {log_file}")

  if __name__ == "__main__":
      main()
  ```

- [ ] **Step 2: Commit the runner changes**

  Run:
  ```bash
  git add src/run_experiments.py
  git commit -m "feat: update run_experiments.py with dual verification, timing updates, and metrics columns"
  ```
