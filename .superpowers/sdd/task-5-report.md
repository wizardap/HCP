# Task 5 Report: Full Regression & Benchmark Results

## Summary

The full 18-graph FHCPP benchmark was executed with `--incremental --cycle auto --time-limit 120`.

### Benchmark Table (120s Time Limit)

| Graph | Variables | Clauses | Total Run (s) | Total Solve (s) | Final Solve (s) | Status | Verified |
|---|---|---|---|---|---|---|---|
| **graph48** | 6,172 | 23,833 | 42.24s | 12.00s | 0.00s | **SAT** | Yes |
| **graph162** | 828,552 | 3,309,667 | 74.42s | 0.18s | 0.01s | **SAT** | Yes |
| **graph171** | 15,938 | 143,444 | 19.66s | 19.37s | 19.37s | **SAT** | Yes |
| **graph197** | 19,010 | 171,092 | 16.14s | 15.74s | 15.74s | **SAT** | Yes |
| **graph223** | 7,980 | 28,012 | 96.99s | 66.19s | 1.79s | **SAT** | Yes |
| **graph237** | 23,618 | 212,564 | 16.19s | 15.76s | 15.76s | **SAT** | Yes |
| **graph249** | 24,930 | 224,372 | 8.99s | 8.48s | 8.48s | **SAT** | Yes |
| **graph252** | 25,154 | 226,388 | 25.92s | 25.38s | 25.38s | **SAT** | Yes |
| **graph254** | 25,314 | 227,828 | 7.45s | 6.93s | 6.93s | **SAT** | Yes |
| **graph255** | 27,390 | 275,683 | 17.32s | 16.65s | 16.65s | **SAT** | Yes |
| **graph424** | 50,202 | 175,088 | 119.60s | 118.04s | 0.01s | **SAT** | Yes |
| **graph446** | 52,560 | 184,119 | 114.20s | 112.12s | 0.01s | **SAT** | Yes |
| **graph470** | 69,924 | 240,411 | 120.90s | 118.72s | 0.41s | TIMEOUT | N/A |
| **graph491** | 178,014 | 593,044 | 99.74s | 92.03s | 0.04s | **SAT** | Yes |
| **graph506** | 184,958 | 616,918 | 121.60s | 116.89s | 7.63s | TIMEOUT | N/A |
| **graph522** | N/A | N/A | 120.00s | N/A | N/A | TIMEOUT | N/A |
| **graph526** | 194,086 | 646,286 | 129.13s | 115.95s | 12.85s | TIMEOUT | N/A |
| **graph529** | 206,830 | 690,410 | 114.43s | 101.91s | 0.04s | **SAT** | Yes |

## Impact Assessment

- **Gomory-Hu Tree Integration:** Successfully prioritizes minimum cuts on contracted component graphs.
- **Model Extraction Optimization & Allocation Reuse:** Extracted edge variables only and reused memory buffers in `SecEncoder`, reducing overhead.
- **Order-Independent Oscillation Hash:** Enables accurate tracking of oscillation cycles across iterations.
- **Sound 2-Component Deadlock Fix:** Correctly avoids spurious UNSAT errors by applying sound DFJ cycle blocking.
