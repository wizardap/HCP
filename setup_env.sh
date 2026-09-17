#!/usr/bin/env bash
set -e

echo "================================================================="
echo "HCP SOLVER ENVIRONMENT SETUP SCRIPT"
echo "================================================================="

echo "[*] Step 1: Installing system build dependencies (Debian/Ubuntu)..."
if command -v apt-get &> /dev/null; then
    sudo apt-get update -y
    sudo apt-get install -y build-essential python3-dev python3-pip python3-venv zlib1g-dev git
fi

echo "[*] Step 2: Installing Python packages..."
python3 -m pip install --upgrade pip --break-system-packages 2>/dev/null || python3 -m pip install --upgrade pip
python3 -m pip install -r requirements.txt --break-system-packages 2>/dev/null || python3 -m pip install -r requirements.txt

echo "[*] Step 3: Verifying CaDiCaL SAT Solver bindings..."
python3 -c "from pysat.solvers import Cadical195; s = Cadical195(); s.add_clause([1, 2]); assert s.solve() == True; print('[*] CaDiCaL 1.9.5: Verified and Working!')"

echo "[*] Step 4: Verifying official 29-graph benchmark..."
python3 scratch/verify_29.py

echo "================================================================="
echo "[*] SUCCESS! Environment is fully ready on this machine."
echo "================================================================="
