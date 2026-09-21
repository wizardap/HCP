#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$DIR/src/cegar-fix/target/release/cegar-fix"
if [ ! -f "$BINARY" ]; then
    echo "Building cegar-fix in release mode..."
    cargo build --manifest-path "$DIR/src/cegar-fix/Cargo.toml" --release
fi
exec "$BINARY" "$@"
