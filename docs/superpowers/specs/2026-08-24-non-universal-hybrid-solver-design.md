# Design Specification: Non-Universal Timeout Hybrid HCP Solver

- **Date:** 2026-08-24
- **Branch:** `feat/backbone-freezer-and-subcycle-absorber`
- **Target Repository:** `src/cegar-fix` in `/home/ubuntu/HCP`
- **Status:** APPROVED FOR IMPLEMENTATION

---

## 1. Executive Summary & Goals

The goal of this system is to provide a deterministic, mathematically sound, and scalable Hamiltonian Cycle Problem (HCP) solver capable of systematically solving the **Non-Universal Timeout instances (Class B1, Class B2b, Class A)** from the Flinders Hamiltonian Cycle Project (FHCPCS) within a single-thread budget of $\le 1,800$ seconds per instance.

### Core Objectives:
1. **Reduce CDCL Increment Count on Class B2b Graphs:**
   - On medium-density, girth $\ge 5$ graphs (e.g., `graph566`, `graph651`, `graph734`, `graph766`), the standard single-pair 2-opt leaves fragmented small cycles, requiring 80–115 SAT increments (~2,600s on typical CPUs).
   - Implement a **Multi-Cycle Alternating Chain Absorber** in polynomial time ($O(|E|)$) to chain small cycles together and absorb them into the giant cycle, reducing required SAT increments to $\le 20$.
2. **Automated Topology Profiling & Track Dispatching:**
   - Implement `AutoTopologyClassifier` to profile graph invariants ($N, M, M/N, \Delta$, Hub counts) at $t = 0$ ($< 2\text{ms}$) and route automatically to the optimal solving track (`Track B1`, `Track B2b`, or `Track General`).
3. **Seamless Two-Tier Integration for Class B1 (Dense Hubs / Ladders):**
   - Wire the Two-Tier Decomposer (`two_tier_orchestrator`) into the auto-dispatch system for graphs with $\ge 50$ Hubs (e.g., `graph950`, `graph963`, `graph746`).
4. **100% Zero-Tour Injection & Independent Verification:**
   - Retain complete zero-tour injection integrity.
   - Output valid TSPLIB `.hcp` tour files verified independently against raw uncontracted graph adjacency lists.
5. **Hard Wall-Clock Timeout Enforcement:**
   - Enforce internal timeout checks alongside Unix `timeout 1800` compatibility.

---

## 2. System Architecture & Components

```
                               RAW GRAPH INPUT (graph.col)
                                           │
                                           ▼
                     ┌───────────────────────────────────────────┐
                     │     1. Auto-Topology Classifier           │
                     │  • Profiles N, M, M/N, Max Degree, Hubs   │
                     │  • Decides Target Track (< 2ms)           │
                     └─────────────────────┬─────────────────────┘
                                           │
                   ┌───────────────────────┴───────────────────────┐
                   │                                               │
        [Hubs >= 50 && M/N >= 3.0]                      [General / Sparse M/N < 2.5]
                   │                                               │
                   ▼                                               ▼
  ┌─────────────────────────────────┐             ┌─────────────────────────────────┐
  │ 2. Track B1: Two-Tier Solver    │             │ 3. Track B2: Sinz SMT + Absorber│
  │ • Strip & Hub Topology Partition│             │ • Sinz Degree Encoding (-e 1)   │
  │ • Global Demand Coordinator     │             │ • Dual SMT Cut Clauses (-b 3)   │
  │ • Pinpointed Strip Solver       │             │ • Multi-Cycle Chain Absorber    │
  │ • Macro Splicer & 2-Opt Stitch  │             │ • Backbone Assumption Freezer   │
  └────────────────┬────────────────┘             └────────────────┬────────────────┘
                   │                                               │
                   └───────────────────────┬───────────────────────┘
                                           │
                                           ▼
                     ┌───────────────────────────────────────────┐
                     │     4. Tour Verifier & Output Engine      │
                     │  • Verify Exact-2 Degree on all N vertices│
                     │  • Verify Edge Existence in raw Graph     │
                     │  • Export TSPLIB .hcp Tour File           │
                     └───────────────────────────────────────────┘
```

---

## 3. Detailed Component Specifications

### 3.1 `AutoTopologyClassifier` (`src/cegar-fix/src/auto_classifier.rs`)

**Purpose:** Instantly inspects structural properties of graph $G$ and selects the optimal solver track.

**Interface:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetTrack {
    B1LadderTwoTier,      // For ladder / dense hub graphs (e.g., graph950, graph963, graph746)
    B2SinzChainSMT,       // For girth >= 5 sparse/medium graphs (e.g., graph566, graph651, graph734, graph766)
    GeneralCaDiCaL,       // For general / Class A graphs (e.g., graph1, graph339, graph647)
}

pub struct TopologyFeatures {
    pub n: usize,
    pub m: usize,
    pub density: f64,
    pub max_degree: usize,
    pub hub_count: usize,
}

impl AutoTopologyClassifier {
    pub fn extract_features(g: &Graph) -> TopologyFeatures;
    pub fn classify(features: &TopologyFeatures) -> TargetTrack;
}
```

**Classification Logic:**
- If `hub_count >= 50` and `density >= 3.0` $\implies$ `TargetTrack::B1LadderTwoTier`.
- Else if `density <= 2.2` and `n >= 1000` $\implies$ `TargetTrack::B2SinzChainSMT`.
- Else $\implies$ `TargetTrack::GeneralCaDiCaL`.

---

### 3.2 `CycleChainAbsorber` (`src/cegar-fix/src/cycle_chain_absorber.rs`)

**Purpose:** Greedily links small disjoint subcycles into composite chains before performing 2-point and 3-point absorption into the giant cycle.

**Algorithmic Workflow:**
1. **Identification:**
   - Partition input 2-factor cycles into the dominant giant cycle $C_{\text{giant}}$ and a collection of small subcycles $\mathcal{S} = \{C_1, C_2, \dots, C_k\}$.
2. **Small-to-Small Pairwise Chaining:**
   - Iterate over all pairs $(C_a, C_b) \in \mathcal{S} \times \mathcal{S}$.
   - Find if there exists an edge $(u_a, v_a) \in C_a$ and $(u_b, v_b) \in C_b$ such that $(u_a, u_b) \in E(G)$ and $(v_a, v_b) \in E(G)$ without violating degree-2 contracted chain protection.
   - If found, merge $C_a$ and $C_b$ into a composite cycle $C_{ab}$.
   - Repeat until no further pairwise small-to-small merges are possible.
3. **Giant Cycle Multi-Point Absorption:**
   - For each remaining composite small cycle $C_s$, test all cyclic rotations (both forward and reverse orientations).
   - Find two vertices $u_1, u_2 \in C_{\text{giant}}$ connected to the endpoints of $C_s$.
   - Splice $C_s$ into $C_{\text{giant}}$ in $O(|C_{\text{giant}}|)$.
4. **Early Termination on Complete Coverage:**
   - If $|C_{\text{giant}}| == N$, verify tour validity and return `s SATISFIABLE` immediately.

---

### 3.3 `TourVerifier` (`src/cegar-fix/src/tour_verifier.rs`)

**Purpose:** Independent verification of candidate Hamiltonian tours against the uncontracted raw graph.

**Interface:**
```rust
pub struct TourVerifier;

impl TourVerifier {
    pub fn verify_raw_tour(tour: &[i32], raw_g: &Graph) -> Result<(), String>;
    pub fn write_tsplib_hcp(tour: &[i32], graph_name: &str, output_path: &str) -> std::io::Result<()>;
}
```

**Verification Steps:**
1. Length Check: `tour.len() == raw_g.adjacency_list.len()`.
2. Bijective Set Check: Every vertex $1 \le v \le N$ appears exactly once.
3. Raw Adjacency Check: For all $0 \le i < N$, $(tour[i], tour[(i+1)\%N]) \in E(raw\_g)$.

---

### 3.4 CLI & Main Entry Integration (`src/cegar-fix/src/main.rs`, `options.rs`)

- Add `--auto <0|1>` flag (default: `1`).
  - When `--auto 1` is enabled: The solver profiles the graph and automatically runs the best track.
  - When `--auto 0` is specified: The solver respects user-provided flags (`-e, -b, -y, -t, --two-tier`).
- Hard timeout enforcement: Every loop checks `instant.elapsed().as_secs_f64() >= timeout_secs`.

---

## 4. Verification & Testing Strategy

### 4.1 Unit Tests (`src/cegar-fix/tests/`)
- `tests/test_auto_classifier.rs`: Verify feature extraction and classification decisions on synthetic and real graphs.
- `tests/test_cycle_chain_absorber.rs`: Verify multi-hop small cycle chaining ($C_1 \leftrightarrow C_2 \leftrightarrow C_3$) and absorption into $C_{\text{giant}}$.
- `tests/test_tour_verifier.rs`: Verify raw graph tour checking with positive and negative test cases.

### 4.2 Benchmark Verification Targets
- `graph1.col`: Sanity check ($< 2\text{s}$).
- `graph566.col`: Class B2b benchmark ($N = 3,322$).
- `graph734.col`: Class B2b benchmark ($N = 4,142$).
- `graph950.col`: Class B1 benchmark ($N = 6,620$).

---

## 5. Non-Overpromise & Soundness Commitment
- **No Tour Injection:** Absolutely zero importing or referencing of `.tou` files.
- **Pure SAT / SMT Foundations:** Every reduction and cut clause is mathematically proven sound.
- **Single-Core Resource Compliance:** Tested strictly on single core within 1,800s.
