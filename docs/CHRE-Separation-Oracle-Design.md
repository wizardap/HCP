Title: Chinese Remainder Encoding with Separation Oracle

## Overview

Optimize the current Chinese Remainder Encoding (CRE) approach for Hamiltonian Cycle SAT by implementing a separation oracle that generates connectivity cuts instead of cycle-specific constraints.

## Problem

The current implementation in HcpEncoder.cpp generates redundant constraints based on every possible cycle length modulo:

- `if ((cycle % 3) == 0)` - forbids cycles divisible by 3  
- `if ((cycle % 5) == 0)` - forbids cycles divisible by 5
- `if ((cycle % 7) == 0)` - forbids cycles divisible by 7
- `if ((cycle % 511) == 0)` - forbids cycles divisible by 511
- etc.

This generates thousands of cycle-blocking clauses, each eliminating only one specific cycle.

## Solution

Replace cycle-specific constraints with **separation oracle** that:

1. **After SAT solve**: Analyzes solution's connected components
2. **For each component S** (|S| < |V|): Generate connectivity cut `Σ x(e) ≥ 2` for `e ∈ δ(S)`
3. **Combine with CRT**: Only generate this when component also satisfies relevant residue pattern
4. **Eliminate entire families**: One cut eliminates thousands of cycles instead of one

## Architecture

```
HcpEncoder
    ├── solve_incremental() - Iterative SAT solving loop
    │   └── encode() - Generates base CNF without Benders cuts
    ├── separation_oracle() - Identifies violated constraints
    │   ├── detect_components() - Find connected components in solution
    │   ├── analyze_residue_patterns() - CRT analysis
    │   └── generate_cuts() - Generate cuts from analysis
    ├── connectivity_cut() - Generate δ(S) ≥ 2 constraint
    │   ├── encode_pb_constraint() - PB encoding for the cut
    │   └── add_to_solver() - Add cut incrementally
    └── hybrid_cut() - Combine connectivity + residue (if applicable)
```

## Files Modified

1. `src/HcpEncoder.cpp` - New encode method with incremental solving
2. `src/AtMostOne/PbLibAtMostOne.hpp` - Enhanced PB encoding support
3. `src/SymmetryBreaking/DefaultSymmetryBreaker.hpp` - Updated for hybrid cuts
4. `src/run_experiments.py` - Updated to run incremental experiments

## Performance Goals

- **Goal 1**: Replace thousands of cycle-specific clauses with few connectivity cuts
- **Goal 2**: One cut eliminates entire families of cycles instead of single cycles
- **Goal 3**: Reduce SAT iterations by orders of magnitude for small m (e.g., m=420)
- **Goal 4**: Maintain correctness while improving scalability

## Technical Details

### Current (Inefficient)
```math
// For each cycle length divisor, generate dedicated clauses:
if ((cycle % 3) == 0) generate_cycle_blocking_clauses_for_all_3_divisible()
if ((cycle % 5) == 0) generate_cycle_blocking_clauses_for_all_5_divisible()
if ((cycle % 7) == 0) generate_cycle_blocking_clauses_for_all_7_divisible()
...
// Result: Thousands of individual cycle constraints
```

### Proposed (Optimized)
```math
// After SAT solve, check components:
if (detected_component_S_exists) {
    // Generate strong cut that eliminates entire family
    generate_connectivity_cut(S, residue_pattern_if_applicable)
}

// δ(S) ≥ 2 eliminates all solutions where S remains isolated
// This includes ALL cycle permutations within S, not just one
```

### Key Components

1. **Component Detection**: DFS/BFS to identify disconnected components from SAT solution
2. **Residue Analysis**: CRT-based filtering of relevant components
3. **Cut Generation**: Pseudo-boolean constraints for cut encoding
4. **Hybrid Integration**: Combine connectivity + residue information for stronger cuts

## Testing Strategy

1. **Unit Tests**: 
   - Component detection accuracy
   - Cut generation logic
   - Constraint encoding correctness

2. **Integration Tests**:
   - End-to-end incremental solving
   - Compare with existing incremental approach
   - Verify solution correctness

3. **Benchmark Tests**:
   - Performance comparison with current approach
   - Scalability assessment for different graph sizes
   - Impact of different m values (small m: 420, 840, 1260)

## Timeline

1. **Week 1**: Implement core separation oracle (component detection)
2. **Week 2**: Add connectivity cut generation
3. **Week 3**: Integrate CRT analysis for hybrid cuts
4. **Week 4**: Comprehensive testing and benchmarking
5. **Week 5**: Documentation and final validation

## Success Criteria

- Correctness: Generate valid CNF constraints that enforce Hamiltonian property
- Performance: Significantly fewer clauses and SAT iterations than current approach
- Maintainability: Clean, testable code with clear separation of concerns
- Documentation: Comprehensive documentation and comments explaining the optimization