# SAT-based-CEGAR / Pure Rust HCP Solver

> **Original Work & Attribution:** This repository is based on and extends the Hamiltonian Cycle Problem (HCP) SAT-based CEGAR solver developed by **Takehide Soh** (Kobe University, Japan) at [`https://github.com/TakehideSoh/SAT-based-CEGAR`](https://github.com/TakehideSoh/SAT-based-CEGAR).

---

## 1. Overview & Guarantees

This repository provides a unified, production-grade Hamiltonian Cycle Problem (HCP) solver implemented in **100% pure native Rust** (`cegar-fix`).

### Key Principles & Guarantees
- **Zero Python Dependencies**: All graph parsing, decomposition, SAT formulation, CDCL CEGAR solving, cycle stitching, and tour verification run entirely within the compiled Rust binary.
- **100% De Novo Computation**: Strictly **zero precomputed caches**, zero pre-calculated cut databases, and zero hardcoded tours. Every cut port, corridor boundary, subcycle elimination clause, and tour reconstruction is computed dynamically at runtime directly from the input graph.
- **Soundness via Independent Verification**: Every generated Hamiltonian cycle is rigorously verified by an internal de novo `TourVerifier` (confirming bijection $V \to \text{cycle}$, exactly $|V|$ distinct vertices, and valid edge adjacency in $G$) before any `s SATISFIABLE` result is declared or tour written.

---

## 2. Multi-Stage Pipeline Architecture

The unified solver executes an integrated, automatic multi-stage solving hierarchy:

```
                      Raw DIMACS Graph (.col)
                                 │
                                 ▼
                     Stage 1: Macro-Decomposition
                   (Challenge Graphs 4k-8k Vertices)
                  - Dynamic Tarjan 2-cut discovery
                  - Subgraph corridor partitioning
                  - Parallel block solving (Rayon)
                  - Global tour lifting & stitching
                                 │
                    [Not Applicable / Fallthrough]
                                 │
                                 ▼
                      Stage 2: General CEGAR
                  - In-Rust CNF CaDiCaL SAT engine
                  - Static 3/4-cycle elimination cuts
                  - Dynamic subcycle cut generation
                  - Tier 2 Full LNS localized repair
                                 │
                        [Iter/Parity Limit]
                                 │
                                 ▼
                  Stage 3: Fallback Contraction
                  - Triangle pruning & forced shortcuts
                  - Safe 2-opt cycle search
                  - Degree-2 contracted CEGAR
                                 │
                                 ▼
                            TourVerifier
               - Validates cycle topology & edge validity
               - Exports standard TSPLIB tour (-o)
```

1. **Stage 1 (Macro-Decomposition)**:
   For massive challenge instances ($4,000 \le |V| \le 8,613$), dynamically discovers 2-cut articulation ports using linear-time Tarjan bi-connected component analysis. Partitions the graph into independent block corridors, contracts degree-2 paths, solves subproblems concurrently using Rayon threadpools, and lifts block Hamiltonian paths into a unified global tour.
2. **Stage 2 (General CEGAR Pipeline)**:
   Native incremental CaDiCaL SAT engine (`rustsat` binding) with 1-in-1-out degree constraints, static cycle cuts, dynamic cut selection, and localized SAT repair (Tier 2 full Large Neighborhood Search corridor expansion).
3. **Stage 3 (Fallback Contraction & Pruning)**:
   If general CEGAR encounters hardness plateaus or parity barriers, Stage 3 applies triangle pruning, forced degree-2 shortcut contraction, and safe 2-opt search to resolve dense or obstinate topologies.
4. **TourVerifier**:
   Strict validator enforcing $|V|$-cycle correctness, edge existence, and absence of subcycles before final solution output.

---

## 3. Quickstart & Usage

### 3.1 Convenience Runner Script

The repository includes a standalone bash wrapper `run_solver_rust.sh` that automatically builds the release binary if not present and forwards arguments:

```bash
# Make executable (if needed)
chmod +x run_solver_rust.sh

# Solve a single graph
./run_solver_rust.sh -i FHCPCS-col/graph1.col

# Solve and write verified TSPLIB tour to a file
./run_solver_rust.sh -i FHCPCS-col/graph1.col -o output/graph1.tour --timeout 60

# Run batch solving across benchmark ranges
./run_solver_rust.sh --batch 1 50 --workers 4 --timeout 10
```

### 3.2 Direct Cargo Build & Execution

You can also build and run the native binary directly:

```bash
cd src/cegar-fix
cargo build --release

# Run standalone binary
./target/release/cegar-fix -i ../../FHCPCS-col/graph1.col -o tour.txt
```

### 3.3 Command-Line Options

| Option | Description |
|---|---|
| `-i, --input <FILE>` | Input graph in DIMACS `.col` or `.edge` format |
| `-o, --output-tour <FILE>` | Export verified Hamiltonian cycle in standard TSPLIB tour format |
| `--timeout <SECONDS>` | Per-graph timeout in seconds (default: 1800s) |
| `--batch <START> <END>` | Batch mode: solve `graph<START>.col` through `graph<END>.col` from `FHCPCS-col/` |
| `--workers <N>` | Number of concurrent worker threads for batch processing (default: 4) |
| `--checkpoint <FILE>` | Path to store/resume batch execution progress JSON |
| `-e, --encoding <N>` | SAT encoding selector (default: 1) |
| `-b, --subcycle <N>` | Subcycle elimination method (default: 3) |
| `-y, --dynamic <N>` | Dynamic cycle cut strategy (default: 3) |
| `-t, --static <N>` | Static cycle cut strategy (default: 3) |
| `-l, --localized <N>` | Localized SAT repair strategy (default: 1) |

---

## 4. Benchmark Suites

- **Suite A (Standard Verification)**:
  Graphs `graph1` .. `graph50` in `FHCPCS-col/` are routinely verified:
  ```bash
  ./run_solver_rust.sh --batch 1 50 --workers 2 --timeout 10
  ```
  Result: **50/50 solved (100.0%)** with 0 errors.

- **29-Graph Challenge Suite** ($4,000 \le |V| \le 8,613$):
  `710, 717, 746, 788, 832, 868, 882, 937, 944, 950, 951, 954, 959, 960, 963, 965, 966, 971, 974, 975, 976, 981, 982, 983, 987, 990, 993, 994, 998`.
  See [`docs/BENCHMARK_29_HANDOFF.md`](docs/BENCHMARK_29_HANDOFF.md) and [`docs/rust_solver_architecture_and_methods.md`](docs/rust_solver_architecture_and_methods.md) for detailed structural analyses and solver progress.

---

## 5. Citation & Reference

If you use or reference this codebase, please cite the upstream original work:
- **Author:** Takehide Soh (Kobe University, Japan)
- **Upstream Repository:** [https://github.com/TakehideSoh/SAT-based-CEGAR](https://github.com/TakehideSoh/SAT-based-CEGAR)
