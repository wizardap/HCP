#!/usr/bin/env bash
set -e

echo "================================================================="
echo "HCP SOLVER ENVIRONMENT SETUP (PYTHON & RUST)"
echo "================================================================="

# Step 1: System packages (Debian / Ubuntu)
echo "[*] Step 1: Installing system build dependencies..."
if command -v apt-get &> /dev/null; then
    sudo apt-get update -y
    sudo apt-get install -y \
        build-essential \
        gcc \
        g++ \
        make \
        cmake \
        pkg-config \
        libssl-dev \
        libclang-dev \
        clang \
        zlib1g-dev \
        python3 \
        python3-dev \
        python3-pip \
        python3-venv \
        git \
        curl
fi

# Step 2: Rust toolchain (if not already installed)
echo "[*] Step 2: Checking Rust toolchain..."
if ! command -v cargo &> /dev/null; then
    echo "[*] Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "[*] Rust is already installed: $(rustc --version)"
fi

# Build native Rust solver
echo "[*] Building native cegar-fix Rust binary..."
cargo build --release --manifest-path src/cegar-fix/Cargo.toml
echo "[*] Rust binary ready at: src/cegar-fix/target/release/cegar-fix"

# Step 3: Python dependencies
echo "[*] Step 3: Installing Python dependencies..."
python3 -m pip install --upgrade pip --break-system-packages 2>/dev/null || python3 -m pip install --upgrade pip
python3 -m pip install -r requirements.txt --break-system-packages 2>/dev/null || python3 -m pip install -r requirements.txt

# Step 4: Verification
echo "[*] Step 4: Verifying CaDiCaL SAT Solver in Python..."
python3 -c "from pysat.solvers import Cadical195; s = Cadical195(); s.add_clause([1, 2]); assert s.solve() == True; print('[*] PySAT CaDiCaL 1.9.5: Verified and Working!')"

echo "[*] Step 5: Verifying official 29-graph benchmark..."
python3 scratch/verify_29.py

echo "================================================================="
echo "[*] SUCCESS! Full Python + Rust environment is ready!"
echo "================================================================="
