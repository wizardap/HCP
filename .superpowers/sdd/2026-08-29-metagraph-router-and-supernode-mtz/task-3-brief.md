# Task 3 Brief: Benchmark Verification on `graph479.col` & `graph668.col`

## Overview
Build release binary and run benchmark verification on `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col` to verify `MetagraphRouter` module detection and Supernode MTZ clause injection at Round 0.

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Commands use `taskset -c 0,1,2 nice -n 19` (Core 3 reserved for user).
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- $T_{\max} = 1800\text{s}$.

## Steps
1. Build release binary: `taskset -c 0,1,2 nice -n 19 cargo build --release`
2. Run full workspace test suite: `taskset -c 0,1,2 nice -n 19 cargo test`
3. Run benchmark on `FHCPCS-col/graph479.col` for up to 60s to observe `MetagraphRouter` module detection and Supernode MTZ clauses.
4. Document the execution output in your report `/home/ubuntu/HCP/.superpowers/sdd/2026-08-29-metagraph-router-and-supernode-mtz/task-3-report.md`.
