# AGENTS.md — SnarkTerm AI Agent Guide

## Project Identity

- **Project:** SnarkTerm — A GPU-accelerated Linux terminal emulator with a sarcastic AI personality
- **Language:** Rust (edition 2021, workspace with 12 crates)
- **Version:** 0.1.0-alpha.0
- **Repository:** https://github.com/ghostfrogcherry/snarkterm

## Architecture Constraints

See `docs/ARCHITECTURE.md` for full detail. Core principle: personality features must never affect terminal correctness.

Key constraints:
- Commentary never enters PTY input or output
- Commentary rendered only in a dedicated gutter/overlay
- Personality, Ollama, plugins, persistence must not block PTY or render hot paths
- Terminal continues functioning if personality systems fail

## Workspace Crates

| Crate | Purpose |
|-------|---------|
| `snarkterm-app` | CLI, GPU window, keyboard input |
| `snarkterm-core` | Terminal grid, CSI/SGR parser, types |
| `snarkterm-pty` | PTY abstractions, shell integration |
| `snarkterm-personality` | Commentary generation |
| `snarkterm-rules` | Danger command detection |
| `snarkterm-plugins` | Plugin manifest/permissions (WASM) |
| `snarkterm-config` | TOML config schema |
| `snarkterm-db` | SQLite stats/achievements |
| `snarkterm-llm` | Ollama client |
| `snarkterm-render` | Render abstractions |
| `snarkterm-ui` | UI component types |
| `snarkterm-testkit` | Test utilities |

## Validation Commands

```bash
# Fastest safe check (preferred):
cargo check --workspace

# Full validation suite:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

# Security:
cargo deny     # if installed
cargo audit    # if installed
```

## Safe AI Workflow

1. Read `AGENTS.md`, `PROJECT_STATUS.md`, `docs/ARCHITECTURE.md` first
2. Read specific files before editing
3. Make smallest possible change
4. Run `cargo check --workspace` after changes
5. Never commit unless explicitly asked
6. Never install dependencies globally
7. Never delete/rename existing files

## Security-Sensitive Areas

See `SECURITY.md` and `SECURITY_REVIEW.md`:
- PTY handling, shell integration hooks, clipboard escape sequences
- Plugin sandboxing, WASM runtime
- Ollama prompt construction and command redaction
- Local command history storage in SQLite
- wgpu GPU rendering (shader safety)

## Important Paths

| Path | Purpose |
|------|---------|
| `Cargo.toml` | Workspace manifest |
| `crates/*/` | 12 crate packages |
| `docs/` | Architecture, design, roadmap |
| `assets/` | Bundled assets |
| `examples/` | Config examples, shell integration |
| `man/` | Man pages |
| `tldr/` | TLDR pages |
| `.github/workflows/ci.yml` | CI configuration |
