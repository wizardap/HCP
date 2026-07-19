# SEC Loop Cut-Strengthening & Multi-Cut Batching Design

## Goal
Improve total runtime of the Hamiltonian Cycle Problem (HCP) SAT solver when running with Chinese Remainder Encoding (CRE) cycle length $c = 1$. With $c = 1$, formula size is minimal (~1.7k–9.5k variables for 500-node graphs), but intermediate Subtour Elimination Constraint (SEC) iterations consume >95–99% of total runtime due to search space oscillation. This design strengthens intermediate SEC cuts and batches multi-cuts to dramatically accelerate loop convergence.

---

## 1. Context & Bottleneck Analysis

From benchmark analysis of `sol_optimized_cre_c1.csv`:
- **Total Solver Time vs Final Solve Time**: Intermediate incremental SEC calls account for virtually all solver runtime (e.g. `graph424`: 79.62s intermediate solver time vs 0.0087s final solve time).
- **Subtour Oscillation**: Under $c = 1$, subcycles frequently rearrange internally within large component subgraphs. Standard single-component SEC clauses only enforce that the entire component must have at least one outgoing and incoming edge, allowing CaDiCaL to explore many intermediate non-Hamiltonian configurations inside component subgraphs.
- **Timeouts**: `graph470` and `graph491` timed out at 600s under standard $c = 1$ SEC loop.

---

## 2. Architecture & SEC Cut Strengthening Strategy

```
+--------------------------+        Current Assignment        +----------------------+
|  Incremental SAT Solver  | -------------------------------> |   SubtourDetector    |
|        (CaDiCaL)         |                                  | (Extract Components) |
+--------------------------+                                  +----------------------+
             ^                                                           |
             |                                                           v
             |     Batched Clauses                            +----------------------+
             +----------------------------------------------- |      SecEncoder      |
                                                              +----------------------+
                                                                 |
                                                                 +--> Primary Boundary SEC Clauses
                                                                 +--> Subcomponent Min-Cut Splitting
                                                                 +--> Small Cycle DFJ (|S| <= 3)
```

### 2.1 Component Processing Strategy

For each detected component $S \subset V$:

1. **Fast Literal Collection ($O(\text{deg}(S))$ complexity)**:
   - Outgoing clause: $\bigvee_{u \in S, v \notin S} e_{u \to v}$
   - Incoming clause: $\bigvee_{u \notin S, v \in S} e_{u \to v}$
   - Eliminate full-graph $O(V \cdot \bar{d})$ scanning by using direct adjacency lookups.

2. **Small Cycle DFJ ($\le 3$ vertices)**:
   - For components of size $|S| \le 3$, add full edge-negation clauses ($\neg e_1 \lor \dots \lor \neg e_k$) alongside SEC boundary clauses to prune trivial small cycles instantly.

3. **Subcomponent Internal Cut-Splitting ($|S| > 3$)**:
   - Compute internal edge connectivity / minimum edge cuts within the induced subgraph $G[S]$.
   - If a internal bottleneck cut of weight $\le 2$ partitions $S$ into sub-components $A$ and $B$, generate outgoing and incoming SEC clauses for $A$ and $B$ simultaneously in the current iteration.
   - **Rationale**: Cuts off intermediate cycle rearrangements inside large subgraphs before CaDiCaL wastes search decisions on them.

---

## 3. Class Interfaces & Data Flow

### 3.1 `SecEncoder` Modifications (`src/SecEncoder.hpp`)

```cpp
#ifndef SEC_ENCODER_HPP
#define SEC_ENCODER_HPP

#include <vector>
#include "SubtourDetector.hpp"

class Graph;

class SecEncoder {
public:
    explicit SecEncoder(const Graph& graph);

    // Encodes primary SEC boundary clauses, small-cycle DFJ clauses, and subcomponent min-cuts
    std::vector<std::vector<int>> encodeSecs(
        const std::vector<Component>& components,
        bool enableSubcutSplitting = true
    );

private:
    const Graph& graph_;

    std::vector<int> getOutgoingLiterals(const Component& component);
    std::vector<int> getIncomingLiterals(const Component& component);
    std::vector<Component> findInternalSubcuts(const Component& component);
};

#endif // SEC_ENCODER_HPP
```

---

## 4. Performance Goals & Verification Plan

### 4.1 Target Metrics
- **SEC Loop Iteration Reduction**: 50–80% fewer iterations on complex instances (`graph424`, `graph223`, `graph526`).
- **Timeout Resolution**: Solve `graph470` and `graph491` within the 600s time limit (target $<120$s).
- **Execution Overhead**: Keep graph-side cut generation time $<10\%$ of total solve time.

### 4.2 Verification & Testing
- Run test suite on all 18 FHCPP benchmark graphs.
- Verify solution correctness via `HcpDecoder` (ensure 1-factor and single Hamiltonian cycle).
- Compare total runtime and iteration count against original `sol_optimized_cre_c1.csv`.
