# Design Specification: Staged-Length Multi-Round Lazy SMT Solver for Universal Core HCP

**Date:** 2026-08-22  
**Target Graph:** `FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$) & FHCP Challenge Universal Core instances  
**Author:** Pair Programming (Antigravity & User)  
**Academic Basis:** Extended from the foundational work of Prof. Takehide Soh (Kobe University, Japan) on SAT-based CEGAR for Hamiltonian Cycle Problems.

---

## 1. Overview & Research Rationale

### 1.1 Problem Context
Previous experiments on `graph950.col` revealed two critical failure modes in SAT-based HCP solving:
1. **Whole-Graph Naive CEGAR:** Adding thousands of long Subtour Elimination Clauses (SECs) across all detected cycles at once caused CaDiCaL's Boolean Constraint Propagation (BCP) time to explode exponentially (from 9.0s in Iteration 0 to 95.5s in Iteration 1), causing a hard timeout after only 4–6 iterations.
2. **Two-Tier Strip/Hub Decomposition:** While accelerating outer iterations to ~0.15s via caching, the macro coordinator only assigned port counts ($\text{demand} \in \{1, 2\}$) without awareness of internal endpoint pairings ($h_a \leftrightarrow h_b$). When strip paths were spliced, the tour scrambled into 24–35 disconnected cycles at every iteration.

### 1.2 The Staged-Length Lazy SMT Solution
The **Staged-Length Multi-Round Lazy SMT Solver** keeps SAT as the central solving engine on the whole graph while preventing BCP explosion through:
1. **Length-Staged Cycle Filtering ($K_{\text{stage}}$ Progression):** Prioritizing elimination of small tight cycles ($L \le 2 \to 4 \to 8 \to 16 \dots$) before addressing larger macro-cycles.
2. **Dual Short Clauses per Subcycle:** Generating an ultra-short Direct Cycle Exclusion clause ($\bigvee_{e \in C} \neg x_e$) and a Boundary Cut-Crossing clause ($\bigvee_{e \in \delta(C)} x_e$).
3. **Strict Batch Size Capping ($\le 500$ clauses/round):** Guaranteeing that CaDiCaL's incremental solve time remains bounded within $[0.2\text{s}, 1.5\text{s}]$ per iteration.

---

## 2. Mathematical Formulation & Architecture

```
                       +----------------------------------+
                       | Raw Graph G (6,620v, 28,718e)    |
                       +-----------------+----------------+
                                         |
                                         v
                       +----------------------------------+
                       | Initial CNF Generator (Sinz enc) |
                       | - Exact-2 on all vertices (Sinz) |
                       | - Directed Arc Literals x_(u,v)  |
                       | - Fast Triangle Cuts (3-cycles)  |
                       +-----------------+----------------+
                                         |
                                         v
         +------------>+----------------------------------+<-------------+
         |             | CaDiCaL Incremental SAT Core     |              |
         |             +-----------------+----------------+              |
         |                               |                               |
         |                               v                               |
         |             +----------------------------------+              |
         |             | 2-Factor Subcycle Extractor      |              |
         |             +-----------------+----------------+              |
         |                               |                               |
         |                               v                               |
         |             +----------------------------------+              |
         |             | k == 1 and |C_1| == N?           |              |
         |             +--------+----------------+--------+              |
         |                      |                |                       |
         |                 Yes  |                | No                    |
         |                      v                v                       |
         |             +----------------+ +----------------------------+ |
         |             | Verify on G    | | Staged-Length Filter       | |
         |             | & Output Tour  | | - Select cycles <= K_stage | |
         |             +----------------+ | - Cap <= 500 clauses/round | |
         |                                | - Dual short clauses added |-+
         |                                +----------------------------+
         +-------------------------------- Wall-clock <= 1800s
```

### 2.1 Variables
For every undirected edge $\{u, v\} \in E$, two directed Boolean literals are defined:
- $x_{u \to v} \in \{0, 1\}$
- $x_{v \to u} \in \{0, 1\}$
- Total directed edge variables: $2 \times |E| = 57,436$ variables.

### 2.2 Exact-2 Degree Constraints (Sinz Encoding)
For every vertex $u \in V$:
$$\sum_{v \in N(u)} x_{u \to v} = 1 \quad (\text{out-degree} = 1)$$
$$\sum_{v \in N(u)} x_{v \to u} = 1 \quad (\text{in-degree} = 1)$$
Encoded using **Sinz Sequential Counters** ($\text{AtMost1}$ and $\text{AtLeast1}$) in $O(\deg(u))$ clauses.
- Initial base CNF size: $\approx 340,000$ clauses.

### 2.3 Initial Short-Cycle Pre-Pruning
Fast triangle cuts ($O(|E| \cdot \Delta)$ neighborhood intersection) are added directly into the base CNF:
$$\forall (u, v, w) \text{ forming a triangle}: \quad \neg x_{u \to v} \vee \neg x_{v \to w} \vee \neg x_{w \to u}$$
Eliminates $\approx 35,000$ spurious 3-cycles before the first SAT call.

---

## 3. Staged-Length Lazy SMT & Cut Generation

### 3.1 Stage Progression ($K_{\text{stage}}$)
1. Initialize $K_{\text{stage}} = 2$.
2. In iteration $t$, let $\mathcal{C} = \{C_1, C_2, \dots, C_k\}$ be the set of disjoint directed cycles returned by CaDiCaL.
3. Filter active candidates:
   $$\mathcal{C}_{\text{active}} = \{ C \in \mathcal{C} \mid |C| \le K_{\text{stage}} \}$$
4. If $\mathcal{C}_{\text{active}} = \emptyset$ and $k > 1$:
   - All cycles at or below $K_{\text{stage}}$ have been eradicated.
   - Advance stage: $K_{\text{stage}} \leftarrow \min(2 \times K_{\text{stage}}, N)$.
   - Re-filter $\mathcal{C}_{\text{active}}$ with the new $K_{\text{stage}}$.

### 3.2 Dual Cut Clauses
For each selected cycle $C \in \mathcal{C}_{\text{active}}$ (up to a max batch cap of 500 cycles per iteration):
1. **Direct Cycle Exclusion (No-Good):**
   $$\bigvee_{e \in C} \neg x_e$$
   - Clause length is exactly $|C| \le K_{\text{stage}}$ (e.g. 2–4 literals in early stages).
   - Provides immediate conflict derivation in CaDiCaL's CDCL trail.
2. **Boundary Cut-Crossing:**
   $$\bigvee_{u \in C, v \notin C, (u, v) \in E} x_{u \to v}$$
   - Forces at least one directed edge to leave cycle $C$.

### 3.3 Batch Size Capping
- Max added clauses per iteration: $M_{\text{max}} = 500$.
- Prevents clause database overload and keeps per-iteration solving time bounded within $0.2\text{s} - 1.5\text{s}$.

---

## 4. Resource Governance & Soundness

### 4.1 System Resource Limits
- **CPU Affinity:** Strictly limited to Core 0 & Core 1 (`taskset -c 0,1`).
- **Process Priority:** Lowest priority (`nice -n 19`), ensuring Core 2 & Core 3 remain 100% free for the user at all times.
- **Background Tasks:** Exactly 1 active background task.
- **Memory Footprint:** $\le 350$ MB RAM.
- **Timeout Budget:** Strictly enforced 1800.0s wall-clock timeout.

### 4.2 Independent Verification & Zero Injection
- **Zero Injection Rule:** Absolutely no reading, importing, or referencing `graph950.hcp.tou`.
- **Independent Raw Graph Verification:**
  When a candidate tour $T = (v_1, v_2, \dots, v_n)$ is found:
  1. Verify $|T| == N$ ($6,620$ vertices).
  2. Verify all vertices in $T$ are pairwise distinct.
  3. Verify $(v_i, v_{i+1}) \in E(G)$ for all $i \in 1 \dots n-1$ and $(v_n, v_1) \in E(G)$ using the raw adjacency list of $G$.
  4. Write certified tour to `scratch/graph950/found_tour_staged_smt.hcp`.

---

## 5. Verification & Testing Plan

1. **Unit Tests:**
   - Test Sinz degree-2 encoding on small synthetic graphs ($N=10, N=50$).
   - Test $K_{\text{stage}}$ progression logic (advancing from 2 to 4 to 8).
   - Test dual cut generation (Direct Exclusion + Boundary Crossing).
2. **Benchmark Execution:**
   - Execute staged SMT solver on `FHCPCS-col/graph950.col` with 1800s timeout on Cores 0,1.
   - Monitor iteration times, $K_{\text{stage}}$ progression, and subcycle count reduction.
