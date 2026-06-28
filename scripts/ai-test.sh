#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

if [ ! -f Cargo.toml ]; then
    echo "Error: No Cargo.toml found in $PROJECT_ROOT"
    echo "This script must be run from a Rust project root."
    exit 1
fi

info "Running cargo check --workspace (fastest safe validation)..."
if cargo check --workspace 2>&1; then
    pass "cargo check --workspace passed"
else
    fail "cargo check --workspace failed"
    info "Falling back to: cargo fmt --check && cargo check --workspace"
    echo ""
    cargo fmt --all -- --check 2>&1 || fail "Format check failed"
    cargo check --workspace 2>&1 || fail "Cargo check failed after format"
    exit 1
fi

echo ""
pass "All validation checks passed."
