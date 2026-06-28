# SnarkTerm — Project Status

## Overview

- **Full Name:** SnarkTerm
- **Description:** A GPU-accelerated Linux terminal emulator written in Rust with a sarcastic AI personality that comments on your commands.
- **Main Language:** Rust (edition 2021)
- **Version:** 0.1.0-alpha.0
- **License:** MIT OR Apache-2.0
- **Repository:** https://github.com/ghostfrogcherry/snarkterm

## How to Install/Run/Test

```bash
# Install
cargo install --path crates/snarkterm-app

# Build
cargo build --release

# Run
snarkterm --window    # Native GPU window
snarkterm -c "ls"     # Run command with commentary
snarkterm             # Shell mode in your terminal

# Test
cargo check --workspace   # Fast validation
cargo test --workspace    # Full test suite
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

## Current State

The project is in early architecture and planning stage (pre-alpha). Workspace structure and crate skeletons are defined. No substantial Rust implementation exists yet — see `docs/ARCHITECTURE.md` for planned architecture and `docs/ROADMAP.md` for the phased development plan.

## What Exists

- Rust workspace with 12 crate directories
- CI workflow (fmt, clippy, test, build)
- Architecture and design documents (`docs/`)
- Security policy (`SECURITY.md`)
- Example config and shell integration sketches (`examples/`)
- Schema definition (`docs/schema.sql`)
- Man page and TLDR page (`man/`, `tldr/`)

## What's Missing (Next Steps)

- Actual Rust source code in all crates
- Milestone 0 issues (see `docs/MILESTONE_0_ISSUES.md`)
- Terminal core implementation (PTY, VT parser, grid, rendering)
- Personality/rules engine
- SQLite schema implementation
- Ollama integration
- Plugin system
- Tests for all components
