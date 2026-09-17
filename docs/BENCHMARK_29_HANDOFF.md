# Official 29-Graph HCP Challenge Benchmark: VM Migration & Handoff Guide

> **Scope**: Canonical 29 Challenge Graphs from FHCPCS ($4,000 \le |V| \le 8,613$):
> `710, 717, 746, 788, 832, 868, 882, 937, 944, 950, 951, 954, 959, 960, 963, 965, 966, 971, 974, 975, 976, 981, 982, 983, 987, 990, 993, 994, 998`

---

## 1. Quick Start on a Fresh VM

### System Requirements & Dependencies
- **OS**: Linux (Ubuntu 22.04 / 24.04 recommended)
- **Python**: 3.10+ (tested on Python 3.12 / 3.13)
- **Rust** (optional, only if using `src/cegar-fix`): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Automated 1-Command Setup
```bash
git clone git@github.com:wizardap/HCP.git
cd HCP
git checkout feat/upgrade-hcp-reference
./setup_env.sh
```
This script installs build-essential, python3-dev, pip packages (`requirements.txt`), verifies CaDiCaL, and runs `verify_29.py` to confirm the 11 baseline tours.

### Manual Setup Commands (Step-by-Step)
```bash
# 1. Clone repository
git clone git@github.com:wizardap/HCP.git
cd HCP
git checkout feat/upgrade-hcp-reference

# 2. System dependencies (Debian / Ubuntu)
sudo apt-get update && sudo apt-get install -y build-essential python3-dev python3-pip zlib1g-dev git

# 3. Python dependencies
pip install -r requirements.txt

# 4. Verify that PySAT and CaDiCaL bindings are working
python3 -c "from pysat.solvers import Cadical195; s = Cadical195(); s.add_clause([1, 2]); print('CaDiCaL OK:', s.solve())"

# 5. Run the benchmark verifier on the 11 already solved tours
python3 scratch/verify_29.py
```

---

## 2. Benchmark Status Summary

### Baseline: 11 / 29 Solved (37.9%)
All 11 solved instances are certified sound by `scratch/verify_29.py`:
- `graph710` ($|V|=4,064, |E|=6,800$): `scratch/graph710/found_tour_graph710.hcp`
- `graph717` ($|V|=4,122, |E|=7,638$): `scratch/graph717/found_tour_graph717.hcp`
- `graph746` ($|V|=4,286, |E|=18,286$): `scratch/graph746/found_tour_graph746.hcp`
- `graph788` ($|V|=4,620, |E|=7,560$): `scratch/graph788/found_tour_graph788.hcp`
- `graph882` ($|V|=5,686, |E|=9,306$): `scratch/graph882/found_tour_graph882.hcp`
- `graph944` ($|V|=6,544, |E|=10,925$): `scratch/engine/found_tour_graph944.hcp`
- `graph950` ($|V|=6,620, |E|=28,718$): `scratch/graph950/found_tour_puresat.hcp`
- `graph963` ($|V|=7,020, |E|=30,518$): `scratch/graph963/found_tour_graph963.hcp`
- `graph975` ($|V|=7,420, |E|=32,318$): `scratch/graph975/found_tour_graph975.hcp`
- `graph982` ($|V|=7,620, |E|=33,218$): `scratch/graph982/found_tour_graph982.hcp`
- `graph990` ($|V|=8,020, |E|=35,018$): `scratch/graph990/found_tour_graph990.hcp`

---

## 3. Structural Contraction Matrix of the 18 Remaining Graphs

All 18 unsolved graphs have been profiled and partitioned into 3 distinct structural archetypes:

| Graph | $|V|$ | $|E|$ | Outer Corridors | Comp 0 $|V_0|$ | Contracted $|V_c|$ | Contraction Ratio | Primary Solver Pipeline |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| **graph832** | 5,070 | 7,986 | 0 | 5,070 | **4,148** | 18.2% | `unified_solver.py` |
| **graph868** | 5,544 | 9,072 | 0 | 5,544 | **1,848** (dir) | 66.7% | `directed_solver.py` |
| **graph937** | 6,412 | 10,762 | 0 | 6,412 | **4,412** | 31.2% | `unified_solver.py` |
| **graph951** | 6,630 | 10,578 | 0 | 6,630 | **5,092** | 23.2% | `unified_solver.py` |
| **graph954** | 6,735 | 11,504 | 0 | 6,735 | **4,735** | 29.7% | `unified_solver.py` |
| **graph959** | 6,925 | 11,727 | 0 | 6,925 | **5,081** | 26.6% | `unified_solver.py` |
| **graph960** | 6,930 | 11,340 | 0 | 6,930 | **2,310** (dir) | 66.7% | `directed_solver.py` |
| **graph965** | 7,102 | 11,680 | 2 (185v) | 6,917 | **5,071** | 28.6% | `unified_solver.py` |
| **graph966** | 7,104 | 11,952 | 2 (523v) | 6,581 | **4,735** | 33.3% | `unified_solver.py` |
| **graph971** | 7,295 | 11,907 | 1 (15v) | 7,280 | **6,359** | 12.8% | `unified_solver.py` |
| **graph974** | 7,418 | 11,885 | 0 | 7,418 | **5,572** | 24.9% | `unified_solver.py` |
| **graph976** | 7,434 | 11,910 | 0 | 7,434 | **5,588** | 24.8% | `unified_solver.py` |
| **graph981** | 7,620 | 12,251 | 0 | 7,620 | **5,620** | 26.2% | `unified_solver.py` |
| **graph983** | 7,650 | 12,234 | 0 | 7,650 | **5,804** | 24.1% | `unified_solver.py` |
| **graph987** | 7,850 | 12,408 | 0 | 7,850 | **6,312** | 19.6% | `unified_solver.py` |
| **graph993** | 8,380 | 13,329 | 0 | 8,380 | **6,534** | 22.0% | `unified_solver.py` |
| **graph994** | 8,401 | 13,828 | 2 (354v) | 8,047 | **6,049** | 28.0% | `unified_solver.py` |
| **graph998** | 8,613 | 14,352 | 2 (185v) | 8,428 | **6,584** | 23.6% | `unified_solver.py` |

---

## 4. Execution Guide & How to Solve Remaining Graphs

### Pipeline A: Generic Unified Solver (`scratch/engine/unified_solver.py`)
Suitable for:
- **Corridor graphs**: `graph965, 966, 971, 994, 998`.
- **Pure degree-2 contracted graphs**: `graph832, 937, 951, 954, 959, 974, 976, 981, 983, 987, 993`.

**Run Command**:
```bash
mkdir -p scratch/graphXXX
python3 scratch/engine/unified_solver.py FHCPCS-col/graphXXX.col scratch/graphXXX/found_tour_graphXXX.hcp
```

**What it does automatically in memory**:
1. Stage 1: Bridges & corridor detection (extracts outer 2-port subgraphs).
2. Stage 2: Solves outer corridors via micro-SAT HP (`sat_merger.py`) in $<2$ seconds, replacing them with single virtual shortcut edges.
3. Stage 3: Comp 0 degree-2 recursive chain contraction (reduces 1,000-2,000 vertices).
4. Stage 4: Pure direct combinatoric degree encoding + chordless triangle/square cuts + incremental CaDiCaL CEGAR loop + multi-cycle closing operator.
5. Stage 5: Unrolls macro-chains and splices corridor paths into certified full tour.

### Pipeline B: Directed 3-Block Solver (`scratch/engine/directed_solver.py`)
Suitable for:
- `graph868` ($N=5,544 \to N_{dir}=1,848$)
- `graph960` ($N=6,930 \to N_{dir}=2,310$)

**Run Command**:
```bash
mkdir -p scratch/graph868
python3 scratch/engine/directed_solver.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp
```

### Pipeline C: Verifying New Tours
After any run completes and writes a tour file, register its path in `CANONICAL_TOURS` inside `scratch/verify_29.py` and run:
```bash
python3 scratch/verify_29.py
```

---

## 5. Architectural Invariants & Scientific Discoveries

1. **Pure Direct Combinatoric Degree Encoding (0 Auxiliary Variables)**:
   - `CardEnc.equals(lits, 2)` generates $>25,000$ auxiliary variables in 5,000-vertex graphs, causing CaDiCaL to branch on pseudo-variables and causing artificial UNSAT/stalls.
   - Replaced by:
     - At-most-2: $\binom{d}{3}$ clauses $(\neg x_a \lor \neg x_b \lor \neg x_c)$.
     - At-least-2: $\binom{d}{d-1}$ clauses.
   - For $d \le 5$, this produces at most 15 short clauses per vertex and **0 auxiliary variables**.

2. **Strict Incremental CaDiCaL Guarantee**:
   - No cold restarts, no mid-CEGAR reseeding, no external heuristics resetting CaDiCaL internal learned clauses or VSIDS activity scores.

3. **Cocycle Cut Guard**:
   - Cocycle cuts ($\sum_{e \in \delta(S)} x_e \ge 2$) are applied **ONLY** to subcycles with $|C| \le |V| // 2$.
   - The Giant component ($|C| > |V| // 2$) must NEVER receive positive boundary cuts, only negative cycle cuts $(\sum_{e \in C} \neg x_e \ge 1)$.

4. **Multi-Cycle Closing Operator**:
   - For $\le 60$ remaining cycles, the solver runs local SAT HP window splicing (`sat_merge_cycles`) and 2-opt/3-opt absorption before querying CaDiCaL, accelerating convergence by orders of magnitude.
