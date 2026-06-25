# Incremental SEC for Hamiltonian Cycle SAT Solver

**Date:** 2026-06-25  
**Status:** Approved for Implementation

---

## 1. Problem Statement

The current `hcp-solver` performs one-shot CNF encoding with Chinese Remainder Encoding (CRE) and invokes cadical externally. When the CRT modulus `m` is small relative to graph size, many subtours survive the CRT constraints, causing exponential explosion in SAT iterations if using naive cycle-blocking clauses.

**Solution:** Implement lazy Subtour Elimination Constraints (SECs) via incremental SAT solving with cadical's C API, adding directed cutset constraints for each disconnected component found in candidate solutions.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    IncrementalSolver                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Variable    │  │   cadical    │  │  SEC Encoder     │  │
│  │  Manager     │  │  (C API)     │  │  (Seq. Counter)  │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                     │            │
│         ▼                 ▼                     ▼            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Solve Loop                           │   │
│  │  1. Encode base constraints (degree + CRT + sym)      │   │
│  │  2. ccadical_solve()                                  │   │
│  │  3. Extract model → detect components (Union-Find)    │   │
│  │  4. If Hamiltonian → return SAT                       │   │
│  │  5. For each component S: add directed SECs           │   │
│  │     Outgoing: Σ_{u∈S,v∉S} x_{u,v} ≥ 1                 │   │
│  │     Incoming: Σ_{u∉S,v∈S} x_{u,v} ≥ 1                 │   │
│  │  6. Repeat until SAT/UNSAT/TIMEOUT (600s)             │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Component Specifications

### 3.1 VariableManager (`src/VariableManager.hpp/.cpp`)

PbLib-style auxiliary variable manager ensuring single allocation namespace across base encoding and incremental SECs.

```cpp
class VariableManager {
public:
    explicit VariableManager(int32_t firstFree = 1);
    int32_t newVar();                    // Allocate fresh variable
    void freeVar(int32_t v);             // Recycle variable
    void freeVars(int32_t start, int32_t end);
    int32_t getMaxVar() const;
    void resetTo(int32_t newFirstFree);  // For testing
private:
    int32_t nextVar_;
    std::unordered_set<int32_t> freeVars_;
};
```

- `HcpEncoder` uses this for all base variables (edge vars, CRT counters)
- `SecEncoder` uses this for SEC auxiliary variables
- Prevents variable ID collisions between base and incremental clauses

### 3.2 IncrementalSolver (`src/IncrementalSolver.hpp/.cpp`)

Thin wrapper around cadical C API (`ccadical.h`) for incremental solving.

```cpp
class IncrementalSolver {
public:
    explicit IncrementalSolver(int64_t timeLimitMs = 600000);
    ~IncrementalSolver();
    
    void addClause(const std::vector<int>& clause);
    void addClauses(const std::vector<std::vector<int>>& clauses);
    
    enum Result { SAT, UNSAT, TIMEOUT };
    Result solve();
    
    int getModelValue(int lit) const;    // 1 (true), -1 (false), 0 (unassigned)
    std::vector<int> getModel() const;   // Full model as literal values
    int getNumVars() const;
    
    void setTimeLimit(int64_t ms);
private:
    CCaDiCaL* solver_;
    int64_t timeLimitMs_;
    int64_t startTime_;
    bool interrupted_;
};
```

**Key methods:**
- `addClause()` → `ccadical_add()` per literal, `0` terminates
- `solve()` → `ccadical_solve()`, checks elapsed time
- `getModel()` → iterates `1..getNumVars()`, calls `ccadical_val()`

### 3.3 SubtourDetector (`src/SubtourDetector.hpp/.cpp`)

Detects disconnected components in SAT solution using Union-Find.

```cpp
struct Component {
    std::vector<int> vertices;  // 0-indexed vertex IDs
};

class SubtourDetector {
public:
    static std::vector<Component> detect(
        const std::vector<int>& model,
        const Graph& graph,
        const HcpEncoder& encoder  // For edge var → (u,v) mapping
    );
private:
    // Union-Find on positive edge literals
};
```

**Algorithm:**
1. Iterate all edge variables `x_{u,v}` from encoder's variable map
2. If `model[x_{u,v}] > 0`, union(u, v)
3. Collect components; filter `|S| < |V|`
4. Return all non-trivial components

### 3.4 SecEncoder (`src/SecEncoder.hpp/.cpp`)

Generates directed SEC clauses for a component using Sequential Counter encoding.

```cpp
class SecEncoder {
public:
    SecEncoder(VariableManager& vm, const Graph& graph, const HcpEncoder& encoder);
    
    // Returns all SEC clauses for all components (2 clauses per component)
    std::vector<std::vector<int>> encodeSecs(
        const std::vector<Component>& components
    );
private:
    VariableManager& vm_;
    const Graph& graph_;
    const HcpEncoder& encoder_;  // For edge var lookup
    
    std::vector<int> encodeGeq1(const std::vector<int>& lits);
};
```

**Directed SEC for component S:**
- Outgoing: `x_{u1,v1} ∨ x_{u2,v2} ∨ ... ∨ x_{uk,vk} ≥ 1` where `u∈S, v∉S`
- Incoming: `x_{u1,v1} ∨ x_{u2,v2} ∨ ... ∨ x_{um,vm} ≥ 1` where `u∉S, v∈S`

Since bound is `≥ 1`, each is a **single clause** (no auxiliary variables needed). Total: **2 clauses per component**.

### 3.5 HcpEncoder Modifications (`src/HcpEncoder.hpp`)

Split encoding into base (one-time) and incremental parts.

```cpp
class HcpEncoder {
public:
    // NEW: Accept VariableManager
    HcpEncoder(Graph& g, int c, IAtMostOne& amo, ISymmetryBreaker& sym, 
               int sNode, VariableManager& vm);
    
    // NEW: Encode only base constraints (degree + CRT + symmetry)
    void encodeBase(IncrementalSolver& solver);
    
    // NEW: Accessor for edge variable mapping
    int getEdgeVar(int u, int v) const;  // Returns variable for u→v
    const std::vector<std::vector<int>>& getEdgeVars() const;
    
    // EXISTING: Keep for backward compatibility (one-shot mode)
    void encode();  // Outputs full CNF to stdout
private:
    VariableManager& vm_;
    std::vector<std::vector<int>> edgeVars_;  // edgeVars[u][v] = var for u→v
    // ... existing fields
};
```

### 3.6 Solver Integration (`src/Solver.hpp/.cpp`)

```cpp
class Solver {
public:
    // EXISTING
    bool run();  // One-shot encoding mode
    
    // NEW: Incremental mode
    bool runIncremental(int64_t timeLimitMs = 600000);
    
private:
    bool runIncrementalImpl(int64_t timeLimitMs);
};
```

**Incremental loop:**
```cpp
bool Solver::runIncrementalImpl(int64_t timeLimitMs) {
    VariableManager vm(1);
    IncrementalSolver isolver(timeLimitMs);
    
    HcpEncoder encoder(graph_, cycle_, *amo_, *sym_, startNode_, vm);
    encoder.encodeBase(isolver);
    
    while (true) {
        auto result = isolver.solve();
        if (result == IncrementalSolver::UNSAT) return false;
        if (result == IncrementalSolver::TIMEOUT) return false;
        
        auto model = isolver.getModel();
        auto comps = SubtourDetector::detect(model, graph_, encoder);
        
        if (comps.empty()) return true;  // Hamiltonian cycle found
        
        SecEncoder secEncoder(vm, graph_, encoder);
        auto secClauses = secEncoder.encodeSecs(comps);
        isolver.addClauses(secClauses);
    }
}
```

---

## 4. CLI Interface

```bash
# Backward compatible: one-shot CNF to stdout
./hcp-solver graph.edge
./hcp-solver graph.edge -c 2 -a default -s min -b default

# New: Incremental solving mode
./hcp-solver graph.edge --incremental
./hcp-solver graph.edge --incremental --time-limit 600
./hcp-solver graph.edge --incremental --time-limit 300 --sec-encoding sequential

# Decode mode (unchanged)
./hcp-solver graph.edge -d solution.sat
```

**New flags:**
- `--incremental` : Run incremental SEC loop instead of emitting CNF
- `--time-limit <seconds>` : Time limit in seconds (default: 600)
- `--sec-encoding <sequential|cardinality>` : SEC encoding (default: sequential)

---

## 5. Build Integration

**CMakeLists.txt / Makefile additions:**
```makefile
# Link against cadical
CADICAL_LIB = refs/cadical/build/libcadical.a
CADICAL_INC = refs/cadical/src

hcp-solver: ... $(CADICAL_LIB)
    g++ ... -I$(CADICAL_INC) -Lrefs/cadical/build -lcadical
```

**Dependencies:** cadical must be built first (`make -C refs/cadical`)

---

## 6. Correctness Guarantees

1. **Soundness:** Every solution satisfies degree constraints + CRT + all added SECs → Hamiltonian cycle
2. **Completeness:** If Hamiltonian cycle exists, loop terminates with SAT (finite components, each SEC cuts at least one component)
3. The directed SEC for S eliminates all solutions where S is isolated.
3. **Termination:** Max 2^|V| components; each iteration adds ≥2 clauses; time limit 600s enforced

---

## 7. Testing Strategy

| Test | Description |
|------|-------------|
| Unit: VariableManager | Allocate/free/recycle correctness |
| Unit: SecEncoder | Clause generation for known components |
| Unit: SubtourDetector | Component detection on synthetic models |
| Integration: Small graphs | 4-6 vertex graphs, compare with brute force |
| Integration: Grid graphs | 4×4, 5×5 grids from `graphs/` |
| Regression: One-shot mode | `--incremental` off produces identical CNF |
| Stress: Timeout | 600s limit respected on hard instances |

---

## 8. Future Extensions (Not in Scope)

- CRT-aware lazy constraints (residue-pattern learning)
- Cardinality Network encoding for `≥ k` SECs
- Parallel component processing
- Clause deletion / subsumption
- Benders-like framework generalization

---

## 9. Implementation Order

1. `VariableManager` - standalone, testable
2. `IncrementalSolver` - cadical wrapper, test with simple SAT
3. `SubtourDetector` - depends on encoder's edge var map
4. `SecEncoder` - depends on VariableManager, Graph, HcpEncoder
5. `HcpEncoder` modifications - add VariableManager, split encode()
6. `Solver::runIncremental()` - wire all components
7. CLI flags + Makefile updates
8. End-to-end tests on `graphs/` corpus

---

## 10. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| cadical API mismatch | Test minimal incremental example first |
| Variable ID collision | Single VariableManager shared by all encoders |
| SEC clause explosion | Directed `≥ 1` = 1 clause each; very small |
| Time limit precision | Check elapsed time before/after each `solve()` |
| Model extraction | Verify `ccadical_val` returns correct polarity |

---