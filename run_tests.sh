#!/bin/bash

echo "========================================================"
echo "RUNNING 10 KEY REGRESSION GRAPHS (120s TIMEOUT)"
echo "========================================================"

REG_GRAPHS=("graph45" "graph132" "graph161" "graph178" "graph183" "graph230" "graph248" "graph313" "graph339" "graph346")

for g in "${REG_GRAPHS[@]}"; do
    echo "--- Testing $g ---"
    START_TS=$(date +%s%N)
    OUT=$(timeout 120 ./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/${g}.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1 2>&1)
    RET_CODE=$?
    END_TS=$(date +%s%N)
    ELAPSED_SEC=$(awk "BEGIN {print ($END_TS - $START_TS)/1000000000}")
    
    if [ $RET_CODE -eq 124 ]; then
        STATUS="TIMEOUT (>120s)"
    else
        STATUS=$(echo "$OUT" | grep -E "^s (SATISFIABLE|UNSATISFIABLE)" | tail -n 1)
    fi
    INCR=$(echo "$OUT" | grep "overall incremented number" | tail -n 1 | awk '{print $NF}')
    SOL_METHOD=$(echo "$OUT" | grep -E "via |hamiltonian cycle found by" | tail -n 1)
    
    echo "Graph: $g | Status: $STATUS | Time: ${ELAPSED_SEC}s | Incr: $INCR | Method: $SOL_METHOD"
done

echo ""
echo "========================================================"
echo "PROFILING DENSE HUB INSTANCES (120s TIMEOUT)"
echo "========================================================"

DENSE_GRAPHS=("graph560" "graph562" "graph584")

for g in "${DENSE_GRAPHS[@]}"; do
    echo "--- Profiling $g ---"
    START_TS=$(date +%s%N)
    OUT=$(timeout 120 ./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/${g}.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1 2>&1)
    RET_CODE=$?
    END_TS=$(date +%s%N)
    ELAPSED_SEC=$(awk "BEGIN {print ($END_TS - $START_TS)/1000000000}")
    
    if [ $RET_CODE -eq 124 ]; then
        STATUS="TIMEOUT (120s limit reached)"
    else
        STATUS=$(echo "$OUT" | grep -E "^s (SATISFIABLE|UNSATISFIABLE)" | tail -n 1)
    fi
    INCR=$(echo "$OUT" | grep "overall incremented number" | tail -n 1 | awk '{print $NF}')
    SOL_METHOD=$(echo "$OUT" | grep -E "via |hamiltonian cycle found by" | tail -n 1)
    
    echo "Graph: $g | Status: $STATUS | Time: ${ELAPSED_SEC}s | Incr: $INCR | Method: $SOL_METHOD"
done
