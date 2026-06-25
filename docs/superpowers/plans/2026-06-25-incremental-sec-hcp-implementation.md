# Incremental SEC for HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement incremental SAT solving with lazy Subtour Elimination Constraints (SECs) for the Hamiltonian Cycle Problem solver, replacing one-shot CNF encoding with an incremental loop that adds directed cutset constraints.

**Architecture:** Integrated C++ solver using cadical C API for incremental solving, with Union-Find for component detection and directed SEC clauses (2 clauses per component: outgoing ≥ 1, incoming ≥ 1).

**Tech Stack:** C++17, cadical C API (ccadical.h), existing HcpEncoder/Graph structures

## Global Constraints

- C++17 language standard
- Maintain backward compatibility (default one-shot mode unchanged)
- Time limit: 600 seconds for incremental solving
- Directed SECs: outgoing ≥ 1 AND incoming ≥ 1 per component
- All non-trivial components get SECs each iteration
- Link against refs/cadical/build/libcadical.a
- Header: refs/cadical/src/ccadical.h
- Default mode unchanged: ./hcp-solver graph.edge > out.cnf
- New mode: ./hcp-solver graph.edge --incremental

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    runIncremental()                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ HcpEncoder   │  │ cadical      │  │ SubtourDetector  │  │
│  │ (one-shot)   │  │ (incremental)│  │ (Union-Find)     │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                     │            │
│         ▼                 ▼                     ▼            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Solve Loop                           │   │
│  │  1. HcpEncoder outputs base CNF                      │   │
│  │  2. Parse CNF → add clauses to IncrementalSolver      │   │
│  │  3. ccadical_solve()                                  │   │
│  │  4. Extract model → detect components                 │   │
│  │  5. If Hamiltonian → return SAT                       │   │
│  │  6. For each component S: add SECs                    │   │
│  │  7. Repeat until SAT/UNSAT/TIMEOUT (600s)             │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Tasks

### Task 1: IncrementalSolver - cadical Wrapper

**Files:**
- Create: `src/IncrementalSolver.hpp`
- Create: `src/IncrementalSolver.cpp`

**Interfaces:**
- Consumes: CCaDiCaL* from cadical library
- Produces:
  - `addClause(std::vector<int> const&)` - adds clause to solver
  - `addClausesFromCnf(std::string const&)` - parses DIMACS CNF and adds all clauses
  - `solve()` - returns Result {SAT, UNSAT, TIMEOUT}
  - `getModelValue(int lit)` - returns 1/-1/0
  - `getNumVars()` - returns number of variables

- [ ] **Step 1: Write IncrementalSolver.hpp**
```cpp
#ifndef INCREMENTALSOLVER_HPP
#define INCREMENTALSOLVER_HPP

#include <cstdint>
#include <vector>
#include <string>
#include <chrono>

// Forward declaration from cadical
extern "C" {
    struct CCaDiCaL;
    CCaDiCaL* ccadical_init(void);
    void ccadical_release(CCaDiCaL*);
    void ccadical_add(CCaDiCaL*, int lit);
    int ccadical_solve(CCaDiCaL*);
    int ccadical_val(CCaDiCaL*, int lit);
    void ccadical_set_terminate(CCaDiCaL*, void* state, int (*terminate)(void* state));
    int ccadical_vars(CCaDiCaL*);
}

class IncrementalSolver {
public:
    enum class Result { SAT, UNSAT, TIMEOUT };
    
    explicit IncrementalSolver(int64_t timeLimitMs = 600000);
    ~IncrementalSolver();
    
    void addClause(const std::vector<int>& clause);
    void addClausesFromCnf(const std::string& cnfFile);
    
    Result solve();
    int getModelValue(int lit) const;
    std::vector<int> getModel() const;
    int getNumVars() const;
    
    void setTimeLimit(int64_t ms);
    
private:
    static int terminateCallback(void* state);
    
    CCaDiCaL* solver_;
    int64_t timeLimitMs_;
    std::chrono::time_point<std::chrono::steady_clock> startTime_;
    bool terminated_;
    int maxVarAdded_;
};

#endif // INCREMENTALSOLVER_HPP
```

- [ ] **Step 2: Write IncrementalSolver.cpp**
```cpp
#include "IncrementalSolver.hpp"
#include <iostream>
#include <fstream>
#include <sstream>
#include <stdexcept>

IncrementalSolver::IncrementalSolver(int64_t timeLimitMs) 
    : solver_(ccadical_init()), 
      timeLimitMs_(timeLimitMs),
      terminated_(false),
      maxVarAdded_(0) {
    if (!solver_) {
        throw std::runtime_error("Failed to initialize cadical solver");
    }
    ccadical_set_terminate(solver_, this, &IncrementalSolver::terminateCallback);
}

IncrementalSolver::~IncrementalSolver() {
    if (solver_) {
        ccadical_release(solver_);
    }
}

void IncrementalSolver::addClause(const std::vector<int>& clause) {
    for (int lit : clause) {
        ccadical_add(solver_, lit);
        int var = lit > 0 ? lit : -lit;
        if (var > maxVarAdded_) maxVarAdded_ = var;
    }
    ccadical_add(solver_, 0);  // terminate clause
}

void IncrementalSolver::addClausesFromCnf(const std::string& cnfFile) {
    std::ifstream file(cnfFile);
    if (!file.is_open()) {
        throw std::runtime_error("Cannot open CNF file: " + cnfFile);
    }
    
    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line[0] == 'c') continue;  // skip comments
        
        if (line[0] == 'p') {
            // Parse header: p cnf vars clauses
            std::istringstream iss(line);
            std::string p, cnf;
            int vars, clauses;
            iss >> p >> cnf >> vars >> clauses;
            continue;
        }
        
        // Parse clause: numbers ending with 0
        std::istringstream iss(line);
        std::vector<int> clause;
        int lit;
        while (iss >> lit && lit != 0) {
            clause.push_back(lit);
        }
        if (!clause.empty()) {
            addClause(clause);
        }
    }
}

IncrementalSolver::Result IncrementalSolver::solve() {
    startTime_ = std::chrono::steady_clock::now();
    terminated_ = false;
    
    int result = ccadical_solve(solver_);
    
    if (terminated_) {
        return Result::TIMEOUT;
    }
    
    switch (result) {
        case 10:  // SAT
            return Result::SAT;
        case 20:  // UNSAT
            return Result::UNSAT;
        default:
            return Result::UNSAT;
    }
}

int IncrementalSolver::getModelValue(int lit) const {
    return ccadical_val(solver_, lit);
}

std::vector<int> IncrementalSolver::getModel() const {
    int numVars = getNumVars();
    std::vector<int> model(numVars + 1, 0);  // 1-indexed
    
    for (int var = 1; var <= numVars; ++var) {
        model[var] = ccadical_val(solver_, var);
    }
    
    return model;
}

int IncrementalSolver::getNumVars() const {
    int cadical_vars = ccadical_vars(solver_);
    return cadical_vars > maxVarAdded_ ? cadical_vars : maxVarAdded_;
}

void IncrementalSolver::setTimeLimit(int64_t ms) {
    timeLimitMs_ = ms;
}

int IncrementalSolver::terminateCallback(void* state) {
    IncrementalSolver* solver = static_cast<IncrementalSolver*>(state);
    auto now = std::chrono::steady_clock::now();
    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        now - solver->startTime_
    ).count();
    
    if (elapsed >= solver->timeLimitMs_) {
        solver->terminated_ = true;
        return 1;
    }
    return 0;
}
```

- [ ] **Step 3: Commit IncrementalSolver**
```bash
git add src/IncrementalSolver.hpp src/IncrementalSolver.cpp
git commit -m "feat: add IncrementalSolver wrapper around cadical with CNF parsing"
```

---

### Task 2: SubtourDetector Implementation

**Files:**
- Create: `src/SubtourDetector.hpp`
- Create: `src/SubtourDetector.cpp`

**Interfaces:**
- Consumes: 
  - Model vector from IncrementalSolver
  - Graph object (for edge variable mapping via graph.getAdj(u,v))
- Produces: `std::vector<Component>` where Component has `std::vector<int> vertices`

- [ ] **Step 1: Write SubtourDetector.hpp**
```cpp
#ifndef SUBTOURDETECTOR_HPP
#define SUBTOURDETECTOR_HPP

#include <vector>
#include "Graph.hpp"

struct Component {
    std::vector<int> vertices;  // 0-indexed vertex IDs
    
    bool operator<(const Component& other) const {
        return vertices.size() < other.vertices.size();
    }
};

class SubtourDetector {
public:
    // Detects non-trivial connected components (size < |V|)
    // Returns sorted by size (smallest first)
    static std::vector<Component> detect(
        const std::vector<int>& model,
        const Graph& graph
    );
    
private:
    static int find(std::vector<int>& parent, int x);
    static void unite(std::vector<int>& parent, int x, int y);
};

#endif // SUBTOURDETECTOR_HPP
```

- [ ] **Step 2: Write SubtourDetector.cpp**
```cpp
#include "SubtourDetector.hpp"
#include <vector>
#include <unordered_map>
#include <algorithm>

int SubtourDetector::find(std::vector<int>& parent, int x) {
    if (parent[x] != x) {
        parent[x] = find(parent, parent[x]);
    }
    return parent[x];
}

void SubtourDetector::unite(std::vector<int>& parent, int x, int y) {
    int rx = find(parent, x);
    int ry = find(parent, y);
    if (rx != ry) {
        parent[ry] = rx;
    }
}

std::vector<Component> SubtourDetector::detect(
    const std::vector<int>& model,
    const Graph& graph
) {
    int nNode = graph.getNodes();
    
    // Initialize Union-Find: each vertex is its own parent
    std::vector<int> parent(nNode);
    for (int i = 0; i < nNode; ++i) {
        parent[i] = i;
    }
    
    // Union vertices connected by selected edges
    // graph.getAdj(u,v) returns the variable for edge u->v
    for (int u = 0; u < nNode; ++u) {
        for (int v = 0; v < nNode; ++v) {
            if (u == v) continue;
            
            int var = graph.getAdj(u, v);
            if (var > 0 && model[var] > 0) {  // edge u->v is selected
                unite(parent, u, v);
            }
        }
    }
    
    // Group vertices by component
    std::unordered_map<int, std::vector<int>> componentsMap;
    for (int i = 0; i < nNode; ++i) {
        int root = find(parent, i);
        componentsMap[root].push_back(i);
    }
    
    // Convert to Component vector, filtering out trivial components (size < nNode)
    std::vector<Component> components;
    for (const auto& kv : componentsMap) {
        const auto& vertices = kv.second;
        if (static_cast<int>(vertices.size()) < nNode) {
            components.push_back({vertices});
        }
    }
    
    // Sort by size (smallest first)
    std::sort(components.begin(), components.end());
    
    return components;
}
```

- [ ] **Step 3: Commit SubtourDetector**
```bash
git add src/SubtourDetector.hpp src/SubtourDetector.cpp
git commit -m "feat: add SubtourDetector for finding disconnected components"
```

---

### Task 3: SecEncoder Implementation

**Files:**
- Create: `src/SecEncoder.hpp`
- Create: `src/SecEncoder.cpp`

**Interfaces:**
- Consumes: Graph& (for edge variable mapping via graph.getAdj(u,v))
- Produces: `std::vector<std::vector<int>>` clauses for SECs
  - For each component: 2 clauses (outgoing ≥ 1, incoming ≥ 1)

- [ ] **Step 1: Write SecEncoder.hpp**
```cpp
#ifndef SECENCODER_HPP
#define SECENCODER_HPP

#include <vector>
#include "Graph.hpp"

class SecEncoder {
public:
    explicit SecEncoder(const Graph& graph);
    
    // Returns SEC clauses for all components (2 clauses per component)
    std::vector<std::vector<int>> encodeSecs(const std::vector<Component>& components);
    
private:
    const Graph& graph_;
    
    // For directed outgoing cut: Σ x_{u,v} ≥ 1 where u∈S, v∉S
    std::vector<int> getOutgoingLiterals(const Component& component);
    // For directed incoming cut: Σ x_{u,v} ≥ 1 where u∉S, v∈S
    std::vector<int> getIncomingLiterals(const Component& component);
};

#endif // SECENCODER_HPP
```

- [ ] **Step 2: Write SecEncoder.cpp**
```cpp
#include "SecEncoder.hpp"
#include <vector>

SecEncoder::SecEncoder(const Graph& graph) : graph_(graph) {}

std::vector<std::vector<int>> SecEncoder::encodeSecs(const std::vector<Component>& components) {
    std::vector<std::vector<int>> allClauses;
    
    for (const auto& component : components) {
        // Outgoing SEC: at least one edge leaves the component
        auto outgoingLits = getOutgoingLiterals(component);
        if (!outgoingLits.empty()) {
            allClauses.push_back(outgoingLits);
        }
        
        // Incoming SEC: at least one edge enters the component
        auto incomingLits = getIncomingLiterals(component);
        if (!incomingLits.empty()) {
            allClauses.push_back(incomingLits);
        }
    }
    
    return allClauses;
}

std::vector<int> SecEncoder::getOutgoingLiterals(const Component& component) {
    // Create a set for fast lookup
    std::vector<bool> inComponent(graph_.getNodes(), false);
    for (int v : component.vertices) {
        inComponent[v] = true;
    }
    
    std::vector<int> literals;
    
    // For each edge u->v where u in component, v not in component
    for (int u : component.vertices) {
        for (int v = 0; v < graph_.getNodes(); ++v) {
            if (!inComponent[v]) {
                int var = graph_.getAdj(u, v);
                if (var > 0) {
                    literals.push_back(var);  // positive literal
                }
            }
        }
    }
    
    return literals;  // For ≥1, this is just a clause of literals
}

std::vector<int> SecEncoder::getIncomingLiterals(const Component& component) {
    // Create a set for fast lookup
    std::vector<bool> inComponent(graph_.getNodes(), false);
    for (int v : component.vertices) {
        inComponent[v] = true;
    }
    
    std::vector<int> literals;
    
    // For each edge u->v where u not in component, v in component
    for (int v : component.vertices) {
        for (int u = 0; u < graph_.getNodes(); ++u) {
            if (!inComponent[u]) {
                int var = graph_.getAdj(u, v);
                if (var > 0) {
                    literals.push_back(var);  // positive literal
                }
            }
        }
    }
    
    return literals;  // For ≥1, this is just a clause of literals
}
```

- [ ] **Step 3: Commit SecEncoder**
```bash
git add src/SecEncoder.hpp src/SecEncoder.cpp
git commit -m "feat: add SecEncoder for generating directed SEC clauses"
```

---

### Task 4: Modify Solver to support incremental mode

**Files:**
- Modify: `src/Solver.hpp`
- Modify: `src/Solver.cpp`

**Interfaces:**
- Consumes: HcpEncoder, IncrementalSolver, SubtourDetector, SecEncoder
- Produces: `runIncremental()` method

- [ ] **Step 1: Add runIncremental declaration to Solver.hpp**
Add after `bool run();`:
```cpp
bool runIncremental(int64_t timeLimitMs = 600000);
```

- [ ] **Step 2: Implement runIncremental in Solver.cpp**
```cpp
bool Solver::runIncremental(int64_t timeLimitMs) {
    // Step 1: Generate base CNF using existing one-shot encoder
    std::string tmpCnf = "incremental_base.cnf";
    
    // Run encoder in one-shot mode to generate CNF
    // (HcpEncoder outputs to stdout, so we redirect to file)
    // For this, we'll call the existing run() but capture output
    // Actually, let's refactor slightly:
    
    // Create a temporary encoder to get the CNF
    {
        std::ofstream outFile(tmpCnf);
        if (!outFile.is_open()) {
            std::cerr << "c Error: Cannot create temporary CNF file\n";
            return false;
        }
        
        // Redirect stdout to capture CNF output
        std::streambuf* oldCoutBuf = std::cout.rdbuf();
        std::cout.rdbuf(outFile.rdbuf());
        
        HcpEncoder encoder(*graph, cycle, *amo, *sym, startNode);
        encoder.encode();
        
        // Restore stdout
        std::cout.flush();
        std::cout.rdbuf(oldCoutBuf);
        outFile.close();
    }
    
    // Step 2: Create incremental solver and load base CNF
    IncrementalSolver solver(timeLimitMs);
    solver.addClausesFromCnf(tmpCnf);
    
    // Remove temporary file
    std::remove(tmpCnf.c_str());
    
    // Step 3: Solve loop with lazy SECs
    int iteration = 0;
    while (true) {
        iteration++;
        std::cerr << "c Iteration " << iteration << ", variables: " << solver.getNumVars() << "\n";
        
        auto result = solver.solve();
        
        if (result == IncrementalSolver::UNSAT) {
            std::cerr << "c UNSAT after " << iteration << " iterations\n";
            return false;
        }
        
        if (result == IncrementalSolver::TIMEOUT) {
            std::cerr << "c TIMEOUT after " << iteration << " iterations\n";
            return false;
        }
        
        // SAT - extract model and check for subtours
        auto model = solver.getModel();
        
        // Detect connected components
        auto components = SubtourDetector::detect(model, *graph);
        
        if (components.empty()) {
            // Hamiltonian cycle found!
            std::cerr << "c HAMILTONIAN found after " << iteration << " iterations\n";
            
            // Output model to file for decoder
            std::string solFile = "solution.sat";
            std::ofstream solOut(solFile);
            solOut << "SATISFIABLE\nv ";
            for (int var = 1; var <= solver.getNumVars(); ++var) {
                if (model[var] > 0) {
                    solOut << var << " ";
                }
            }
            solOut << "0\n";
            solOut.close();
            
            std::cout << "c Solution written to " << solFile << "\n";
            return true;
        }
        
        // Generate and add SEC clauses
        SecEncoder secEncoder(*graph);
        auto secClauses = secEncoder.encodeSecs(components);
        
        std::cerr << "c Adding " << secClauses.size() << " SEC clauses for " 
                  << components.size() << " components\n";
        
        for (const auto& clause : secClauses) {
            solver.addClause(clause);
        }
    }
}
```

- [ ] **Step 3: Modify main.cpp to support --incremental flag**
Add to argument parsing:
```cpp
} else if (arg == "--incremental") {
    incremental = true;
} else if (arg == "--time-limit") {
    if (i + 1 < argc) {
        timeLimit = std::atoi(argv[++i]) * 1000;  // convert to ms
    }
}
```

Add before the one-shot call:
```cpp
if (incremental) {
    return solver.runIncremental(timeLimit) ? 0 : 1;
}
```

- [ ] **Step 4: Commit changes**
```bash
git add src/Solver.hpp src/Solver.cpp src/main.cpp
git commit -m "feat: add incremental solving mode with --incremental flag"
```

---

### Task 5: Build Integration

**Files:**
- Modify: `src/Makefile` or CMakeLists.txt

- [ ] **Step 1: Update Makefile to compile new files and link cadical**

Add to compilation rules:
```makefile
# New source files
SRCS += src/IncrementalSolver.cpp src/SubtourDetector.cpp src/SecEncoder.cpp

# Cadical library
CADICAL_LIB = refs/cadical/build/libcadical.a
CADICAL_INC = refs/cadical/src

# Add to compiler flags
CXXFLAGS += -I$(CADICAL_INC)

# Add to linker flags
LDFLAGS += -Lrefs/cadical/build -lcadical
```

- [ ] **Step 2: Commit build changes**
```bash
git add src/Makefile
git commit -m "feat: update build system for incremental mode"
```

---

### Task 6: Testing and Verification

- [ ] **Step 1: Build the project**
```bash
make -C src clean
make -C src
```

- [ ] **Step 2: Test backward compatibility**
```bash
# Should produce CNF to stdout (unchanged behavior)
./src/hcp-solver graphs/example.edge | head -20
```

- [ ] **Step 3: Test incremental mode**
```bash
# Should run incremental solver and find Hamiltonian cycle
./src/hcp-solver graphs/example.edge --incremental
```

- [ ] **Step 4: Test with larger graphs from graphs/ directory**
```bash
for f in graphs/*.edge; do
    echo "Testing $f..."
    timeout 60 ./src/hcp-solver "$f" --incremental
done
```

- [ ] **Step 5: Verify time limit works**
```bash
# Set very short time limit to test timeout
./src/hcp-solver graphs/hard_instance.edge --incremental --time-limit 5
```

---

## Key Implementation Notes

### Why Keep HcpEncoder Unchanged?

The current `HcpEncoder::encode()` method is complex (442 lines) and directly outputs CNF to stdout. Rather than refactoring this to support incremental mode, we:

1. Use it as-is to generate the base CNF
2. Parse the CNF file to extract clauses
3. Add clauses to IncrementalSolver

This approach:
- Zero risk of breaking existing functionality
- Simpler implementation
- Easier to debug
- The parsing overhead is negligible compared to SAT solving time

### Edge Variable Mapping

The Graph class provides `graph.getAdj(u, v)` which returns:
- The variable number for edge u→v (positive integer)
- 0 if no edge exists

This is used by:
- SubtourDetector: to check if edge is selected in model
- SecEncoder: to collect boundary edge literals

### Directed SEC Encoding

For component S:
- **Outgoing:** `Σ_{u∈S, v∉S} x_{u,v} ≥ 1` → single clause `(x_{u1,v1} ∨ x_{u2,v2} ∨ ... ∨ x_{uk,vk})`
- **Incoming:** `Σ_{u∉S, v∈S} x_{u,v} ≥ 1` → single clause `(x_{u1,v1} ∨ x_{u2,v2} ∨ ... ∨ x_{um,vm})`

Total: 2 clauses per component. No auxiliary variables needed since bound is 1.

### Termination Guarantee

- Each iteration adds ≥ 2 clauses (outgoing + incoming SEC)
- Each clause eliminates at least one solution (the current bad cycle cover)
- With 600s time limit, guaranteed to terminate
- Worst case: UNSAT detected after finite iterations