# SAT-based-CEGAR / HCP Solver

> **Original Work & Attribution:** This repository is based on and extends the Hamiltonian Cycle Problem (HCP) SAT-based CEGAR solver developed by **Takehide Soh** (Kobe University, Japan) at [`https://github.com/TakehideSoh/SAT-based-CEGAR`](https://github.com/TakehideSoh/SAT-based-CEGAR).

---

## 1. 29-Graph Challenge Benchmark Suite

The primary focus of ongoing development is solving the **29 canonical challenge graphs** ($4,000 \le |V| \le 8,613$) from the FHCPCS benchmark:
`710, 717, 746, 788, 832, 868, 882, 937, 944, 950, 951, 954, 959, 960, 963, 965, 966, 971, 974, 975, 976, 981, 982, 983, 987, 990, 993, 994, 998`

- **Current Verified Sound Status**: **11 / 29 Solved (37.9%)**
- **Verification Script**: `python3 scratch/verify_29.py`
- **Handoff & Migration Guide**: [`docs/BENCHMARK_29_HANDOFF.md`](docs/BENCHMARK_29_HANDOFF.md) (Complete instructions for VM setup, dependencies, running solvers, and structural matrix for the 18 remaining graphs).

---

## 2. Overview & Components

- **`scratch/engine/unified_solver.py`**: Generic unified hierarchical solver (corridor detection $\to$ local micro-SAT corridors $\to$ degree-2 chain contraction $\to$ pure combinatoric CEGAR + multi-cycle closing operator $\to$ full tour assembler).
- **`scratch/engine/directed_solver.py`**: Directed 3-block pipeline for bipartite reduced graphs (`graph868`, `graph960`).
- **`src/cegar-fix`**: Native Rust-based Hamiltonian Cycle Solver utilizing in-Rust CNF encoding, incremental CaDiCaL SAT solving (via `rustsat`), multi-level 2-opt/3-opt cycle patching, MTZ stall injection, and degree-2 contraction.
- **`docs/`**: Comprehensive technical architecture documentation, benchmark analyses, and experimental reports:
  - [`docs/BENCHMARK_29_HANDOFF.md`](docs/BENCHMARK_29_HANDOFF.md): Master handoff document and migration guide for the 29-graph benchmark.
  - [`docs/rust_solver_architecture_and_methods.md`](docs/rust_solver_architecture_and_methods.md): Full technical breakdown of all algorithms and CLI flags in the Rust codebase (`cegar-fix`).
  - [`docs/graph950_methods_and_experiments_report.md`](docs/graph950_methods_and_experiments_report.md): Deep-dive research report on methods attempted on `graph950.col`.
  - [`docs/timeout-classification-analysis.md`](docs/timeout-classification-analysis.md): Systematic categorization of timeout dynamics across 1,001 FHCP benchmark instances.
- **`scratch/`**: Prototyping scripts, two-tier decomposition experiments, and verification tools.

---

## 2. Solvers & Libraries

- **CaDiCaL** (<https://github.com/arminbiere/cadical> version 1.9.4 / 1.9.5)
- **rustsat** (<https://github.com/chrjabs/rustsat> version 0.6.1)
- **PySAT** (`pysat.solvers.Cadical195`) for Python-based decomposition prototypes.

---

## 3. Build & Execution

### Build Release Binary
```bash
cd src/cegar-fix
cargo build --release
```

### Execution Example
```bash
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph1.col -e 1 -b 3 -y 3 -t 3 -l 1
```

For full CLI options and algorithmic descriptions, please refer to [`docs/rust_solver_architecture_and_methods.md`](docs/rust_solver_architecture_and_methods.md).

---

## 4. Citation & Reference

If you use or reference this codebase, please cite the upstream original work:
- **Author:** Takehide Soh (Kobe University, Japan)
- **Upstream Repository:** [https://github.com/TakehideSoh/SAT-based-CEGAR](https://github.com/TakehideSoh/SAT-based-CEGAR)
