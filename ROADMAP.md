# SnarkTerm Roadmap

> **Detailed phased roadmap:** `docs/ROADMAP.md`
> **Milestone 0 issues:** `docs/MILESTONE_0_ISSUES.md`

## Short-Term (Alpha: 0.1.x)

- [ ] Implement crate source code (all 12 crates currently have only Cargo.toml)
- [ ] Milestone 0 issues (20 items: CI, config, window, GPU, PTY, parser, grid, keyboard, etc.)
- [ ] Working terminal core: PTY I/O, VT parser, grid rendering, keyboard input
- [ ] Basic personality rules engine with deterministic commentary
- [ ] SQLite persistence for stats and achievements
- [ ] Shell integration scripts for Bash, Zsh, Fish

## Medium-Term (Beta: 0.2.x – 0.9.x)

- [ ] Phase 2: Rendering quality (glyph atlas, font fallback, cursor styles, selection, scrollback)
- [ ] Phase 3: Tabs, splits, UI (layout tree, command palette, themes, keybindings)
- [ ] Phase 4: Command monitoring (OSC hooks, heuristic fallback)
- [ ] Phase 5: Personality engine (profiles, roast intensity, memory, crash isolation)
- [ ] Phase 6: Persistence and stats (achievements, reports, history management)
- [x] CI pipeline: format, clippy, test, build (`cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`)

## Long-Term (Stable: 1.0.0)

- [ ] Phase 7: Local Ollama integration with redaction and fallback
- [ ] Phase 8: WASM plugin system with sandboxing
- [ ] First public alpha release (codename: "It Probably Works On Your Machine")
- [ ] Cross-shell compatibility validation (vttest, tmux, vim, less, htop, fzf)
- [ ] Performance optimization for large output and low latency
- [ ] Plugin ecosystem documentation and API stability

## Documentation Tasks

- [ ] Rustdoc for all public APIs
- [ ] User guide with configuration reference
- [ ] Plugin development guide
- [ ] Shell integration setup guide

## Testing Tasks

- [ ] Unit tests for core parser and grid
- [ ] Integration tests for PTY I/O
- [ ] Property-based tests for VT sequence parsing
- [ ] Personality engine tests (deterministic rules)
- [ ] GPU rendering tests (screenshot comparison if feasible)

## Security Tasks

- [ ] PTY input/output sandboxing review
- [ ] Plugin WASM sandbox (wasmtime capability model)
- [ ] Ollama prompt injection hardening
- [ ] SQLite storage encryption consideration
- [ ] Dependency audit (cargo audit)
- [ ] Clipboard security review
