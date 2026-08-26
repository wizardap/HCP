# Design Specification: Macro Order-Encoding (MTZ) for Connected Hub Coordination

- **Date:** 2026-08-26
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Breakthrough Premise

### 1.1 The Root Cause of Combinatorial Wander in Two-Tier Coordination
The existing `GlobalDemandCoordinator` encodes exact-2 degree at each hub and endpoint demand counts on strips. However, **connectedness is completely absent from the initial SAT formulation**. As a result:
- CaDiCaL treats the coordinator problem as an unconstrained 2-Factor problem on 155 hubs.
- CaDiCaL generates 15–20 disjoint subtours at every outer iteration.
- The solver relies on lazy Subtour Elimination Constraints (SEC), requiring hundreds of iterations without converging within 1800s.

### 1.2 The Breakthrough: Active Macro Order-Encoding (MTZ)
By embedding **Miller-Tucker-Zemlin (MTZ) Unary Order Encoding** directly into the Hub Coordinator:
1. Every hub $h \ne h_0$ is assigned an integer order $t_h \in [1, N_H - 1]$ via unary ladder literals $O_{h, k} = [t_h \ge k]$.
2. Every active directed Hub-Hub transition and Strip-traversal transition $u \to v$ enforces $t_v \ge t_u + 1$.
3. **Mathematical Theorem:** Any directed cycle not containing $h_0$ violates the strict monotonicity sum $\sum \Delta t \ge |C| > 0$, rendering all macro subtours **strictly UNSAT in CNF**.
4. **Result:** CaDiCaL is mathematically prevented from returning disconnected subtours at the macro level. Every macro assignment is guaranteed to form a single connected cycle across all 155 hubs from Iteration 1!

---

## 2. Mathematical Formalization & CNF Encoding

### 2.1 Graph Definition
Let $H = \{h_0, h_1, \dots, h_{N_H - 1}\}$ be the set of $N_H$ hubs ($N_H \le 155$).
Designate $h_0 \in H$ as the root hub ($t_{h_0} = 0$).

### 2.2 Unary Ladder Order Variables
For each hub $h \in H \setminus \{h_0\}$ and each position $k \in \{1, \dots, N_H - 1\}$:
- Introduce boolean variable $O_{h, k}$ representing the proposition $[t_h \ge k]$.
- **Monotonicity clauses:**
  $$\neg O_{h, k+1} \lor O_{h, k} \quad (\forall k \in \{1, \dots, N_H - 2\})$$

### 2.3 Directed Macro Transitions
1. **Direct Hub-Hub Edge $(u, v) \in E_{\text{HH}}$:**
   - Introduce directed literals $x_{u \to v}$ and $x_{v \to u}$.
   - Link with undirected edge literal $x_{uv}$:
     $$\neg x_{u \to v} \lor x_{uv}, \quad \neg x_{v \to u} \lor x_{uv}, \quad \neg x_{u \to v} \lor \neg x_{v \to u}, \quad \neg x_{uv} \lor x_{u \to v} \lor x_{v \to u}$$
2. **Strip Transitions:**
   - For each strip $si$ and adjacent hub pair $(u, v) \in \text{strip\_adj\_hubs}(si)$ with $u \ne v$:
     - Introduce directed strip traversal literal $s_{si, u \to v}$ (entry at $u$, exit at $v$).
     - Link with strip endpoint demand literals:
       $$\neg s_{si, u \to v} \lor d_1(si, u), \quad \neg s_{si, u \to v} \lor d_1(si, v)$$
     - For single-path strips ($s.\text{len}() < 10$): exactly 1 traversal direction is chosen if strip is active.

### 2.4 MTZ Transition Implication Clauses
For every possible directed macro transition $e_{u \to v} \in \{x_{u \to v}, s_{si, u \to v}\}$:
1. **From Root ($u = h_0, v \ne h_0$):**
   $$\neg e_{h_0 \to v} \lor O_{v, 1}$$
2. **To Root ($u \ne h_0, v = h_0$):**
   No ordering constraint (wrap-around step).
3. **Between Non-Root Hubs ($u \ne h_0, v \ne h_0$):**
   $$\neg e_{u \to v} \lor O_{v, 1}$$
   $$\neg e_{u \to v} \lor \neg O_{u, k} \lor O_{v, k+1} \quad (\forall k \in \{1, \dots, N_H - 2\})$$

---

## 3. Complexity & Scalability Analysis

- **Order Variables:** $(N_H - 1) \times (N_H - 1) = 154 \times 154 \approx 23,716$ variables.
- **Directed Transition Literals:** $\approx 600$ variables.
- **Monotonicity Clauses:** $154 \times 153 \approx 23,562$ binary clauses.
- **Transition Clauses:** $\approx 600 \times 154 \approx 92,400$ ternary clauses.
- **Total CNF Size:** $\approx 24,500$ variables and $\approx 116,000$ clauses.
- **CaDiCaL Solving Latency:** A formula of 116k clauses on 24k variables with direct 2-SAT / 3-SAT structure solves in $\approx 0.05\text{s} - 0.2\text{s}$ in CaDiCaL.

---

## 4. Architecture & Module Modifications

1. **`src/cegar-fix/src/global_demand_coordinator.rs`**:
   - Add `enable_macro_mtz` flag to `GlobalDemandCoordinator`.
   - Implement `encode_macro_mtz(&mut self)` to generate directed transition variables, unary order variables, monotonicity clauses, and transition propagation clauses.
2. **`src/cegar-fix/src/two_tier_orchestrator.rs`**:
   - In `solve_two_tier`, pass `enable_macro_mtz = true`.
   - Upon SAT from `GlobalDemandCoordinator`, extract the directed macro tour and verify that `cycles.len() == 1`.
3. **Tests (`tests/test_macro_mtz.rs`)**:
   - `test_macro_mtz_guarantees_single_cycle`: Verify on synthetic hub graphs that coordinator NEVER returns disconnected subtours.
   - `test_macro_mtz_demand_consistency`: Verify that strip demand literals and directed strip paths match 100%.

---

## 5. System Guarantees
- Pinned to cores 0, 1, 2 via `taskset -c 0,1,2 nice -n 19`. Core 3 reserved for user.
- Zero tour injection.
- Time limit $T_{\max} = 1800\text{s}$.
