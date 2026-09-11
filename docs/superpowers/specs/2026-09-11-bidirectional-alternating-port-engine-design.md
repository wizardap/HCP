# Technical Specification: Bidirectional Alternating Port Engine (Tier 2 Multi-Hop Expansion)

**Author:** DeepMind Advanced Agentic Coding Pair  
**Date:** 2026-09-11  
**Status:** Approved for Implementation Planning  
**Target Repository:** `src/cegar-fix/`  
**Target Benchmark:** `FHCPCS-col/graph868.col` ($|V|=5544, |E|=9072$, contracted $|V|=3696$)

---

## 1. Executive Summary & Problem Formulation

### 1.1 Empirical Context & Bottleneck
In the 1,800-second benchmark of `graph868.col` with `AlternatingPortEngine` (Tier 1 greedy + Tier 2 unidirectional BFS depth 4):
- The engine successfully eliminated **61 subcycles dynamically online** across 8 iterations.
- Raw SAT subcycle counts were compressed from **97 down to 51** (-47.4%), and the giant cycle reached **2,886 vertices** (78.1% of the contracted graph).
- **The Remaining Barrier**: At Increment 8, CaDiCaL timed out after 1,090s of search because accumulating 2,375 cut clauses created a CDCL clause database pollution barrier.
- **Root Cause**: Unidirectional BFS with `max_depth = 4` can only detect alternating cycles of length at most 8 edges (4 active + 4 inactive). Subcycles separated by 3 or more intermediate blocks (requiring 10–12 hops) cannot be merged, leaving ~45 fragmented subcycles that must be cut by SAT clauses.

### 1.2 Objective
Upgrade Tier 2 of `AlternatingPortEngine` to a **Bidirectional Alternating BFS (Meet-in-the-middle)** with a **Smallest-Cycle-First** heuristic:
1. Search forward from a source port $p_0$ up to depth $D_{fwd} \le 5$ (10 alternating edges).
2. Search backward from the goal port $p_{goal} = \text{port\_nbr}[p_0]$ (or ports in target cycles) up to depth $D_{bwd} \le 5$.
3. When frontiers intersect at an intermediate port $p_{mid}$, reconstruct a closed simple alternating cycle of up to **10–12 hops** ($20 - 24$ edges) with search complexity bounded to $2 \times O(b^5) \approx 64$ states instead of $O(b^{10}) \approx 1,024$.
4. Prioritize search from smallest cycles (lengths 4, 8, 16) to eliminate isolated fragments early.
5. Guarantee 100% soundness: Degree-2 preservation, Zero Phantom Edges, and Zero Tour Injection.

---

## 2. Architecture & Data Structures

### 2.1 Traversal Graphs & Port States
In `AlternatingPortEngine`, each 2-chain block $b \in \{0 \dots n_{blocks}-1\}$ has two external ports:
- Port $p = \text{Port} \{ \text{block}: b, \text{end}: 0 \}$ and $p' = \text{Port} \{ \text{block}: b, \text{end}: 1 \}$.
- The active 2-factor defines a 1-to-1 involution: $\text{port\_nbr}[p] = q$ where $(p, q)$ is an active external edge.
- The inactive adjacency graph defines available hops: for each port $p$, $\text{inactive\_adj}[p] = \{ q \mid (p, q) \in E(G), q \neq \text{port\_nbr}[p], q.block \neq p.block \}$.

### 2.2 Bidirectional BFS Transitions
For a given cycle $C$ and an active edge $(p_0, p_{goal})$ where $p_{goal} = \text{port\_nbr}[p_0]$:
1. **Forward Frontier ($\mathcal{F}_{fwd}$)**:
   - Start at $p_0$ at depth 0.
   - Transition: from $u$, take an **inactive** edge to $v \in \text{inactive\_adj}[u]$, then take the unique **active** edge to $w = \text{port\_nbr}[v]$.
   - Record predecessor: $\text{parent}_{fwd}[w] = (u, v)$ with $\text{depth}_{fwd}[w] = d+1$.
2. **Backward Frontier ($\mathcal{F}_{bwd}$)**:
   - Start at $p_{goal}$ at depth 0.
   - Transition: from $w$, take the unique **active** edge to $v = \text{port\_nbr}[w]$, then take an **inactive** edge to $u \in \text{inactive\_adj}[v]$.
   - Record predecessor: $\text{parent}_{bwd}[u] = (w, v)$ with $\text{depth}_{bwd}[u] = d+1$.
3. **Collision Detection (Meet-in-the-middle)**:
   - If a port $p_{mid}$ has been reached by both $\mathcal{F}_{fwd}$ and $\mathcal{F}_{bwd}$:
     $$\text{depth}_{fwd}[p_{mid}] + \text{depth}_{bwd}[p_{mid}] \le D_{total}$$
   - Extract forward path from $p_0$ to $p_{mid}$, and backward path from $p_{goal}$ to $p_{mid}$.
   - Splice paths to form a single closed alternating loop.

---

## 3. Detailed Algorithmic Specification

### 3.1 Smallest-Cycle-First Priority
At each repair iteration:
1. Sort current cycles $C_0, C_1, \dots, C_{k-1}$ by length ascending (or prioritize lengths 4, 8, 16).
2. Exclude the largest cycle (giant cycle) from initiating the search; small cycles initiate forward search, while the giant cycle and other cycles serve as target sinks.

### 3.2 Path Splicing & Disjointness Check
Given forward path $(p_0 \xrightarrow{inact} v_1 \xrightarrow{act} w_1 \dots \xrightarrow{act} p_{mid})$ and backward path $(p_{goal} \xrightarrow{act} v'_1 \xrightarrow{inact} u'_1 \dots \xrightarrow{inact} p_{mid})$:
1. **Disjointness Invariant**:
   - Collect all ports traversed: $S_{ports} = S_{fwd} \cup S_{bwd}$.
   - If $|S_{ports}| \neq |S_{fwd}| + |S_{bwd}| - 1$ (where $p_{mid}$ is the only intersection), the walk contains an internal self-loop. Reject candidate.
2. **Edge Difference Sets**:
   - $\text{rem} = \{ (p_0, p_{goal}) \} \cup \{ (v, w) \text{ on active hops} \}$.
   - $\text{add} = \{ (u, v) \text{ on inactive hops} \} \cup \{ (p_{mid, in}, p_{mid, out}) \}$.
3. **Soundness Verification**:
   - Ensure every $e \in \text{add}$ exists in $G$ (`g.adjacency_list`).
   - Ensure every $e \in \text{rem}$ exists in `current_edges`.
4. **Improvement Condition**:
   - Apply swap: $E' = (E \setminus \text{rem}) \cup \text{add}$.
   - Extract resulting cycles $C'$.
   - If $|C'| < |C|$ (or $|C'| == |C|$ and $\max |C'_i| > \max |C_i|$), accept $E'$ and immediately call `repair_tier1_edges` to propagate greedy merges.

---

## 4. Invariants & Safety Guarantees

1. **Zero Tour Injection**:
   - The engine does not inspect, read, or reference any external tour file (`.tou`).
   - All operations are strictly combinatorial flips on the in-memory graph $G$.
2. **Degree-2 Regularity**:
   - Every modified vertex has exactly one incoming edge removed and one incoming edge added. The degree of every vertex remains strictly 2.
3. **No Lasso / No Phantom Edges**:
   - Disjointness check and explicit membership guards prevent $\rho$-shaped lasso walks or non-existent shortcut chords.
4. **Time & Memory Bounds**:
   - Max depth per direction $D \le 6$; frontier sizes $\le 64$ entries.
   - Global repair timeout per CEGAR round: 2,000 ms.

---

## 5. Verification & Testing Plan

### 5.1 Unit & Synthetic Tests (`tests/test_alternating_bidirectional.rs`)
1. `test_bidirectional_finds_10hop_cycle`:
   - Construct a synthetic graph with 2 disjoint cycles separated by a 10-hop alternating corridor.
   - Assert that unidirectional BFS with depth 4 fails to merge, but bidirectional BFS with depth 5 succeeds.
2. `test_soundness_and_zero_phantom_edges`:
   - Run bidirectional repair on synthetic multi-cycle configurations and verify all edges exist in raw $G$.
3. `test_graph868_snapshot_reduction_bidirectional`:
   - Test on the 31-cycle checkpoint (`scratch/graph868_giant_1686_full_edges.txt`).
   - Verify that bidirectional repair executes in $< 100$ms and achieves equal or better cycle reduction without phantom edges.

### 5.2 Full Non-Regression Suite
- Run `cargo test --tests` to verify 100% pass rate across all existing 52+ tests.
- Verify `cargo build --release` compiles with 0 errors.
