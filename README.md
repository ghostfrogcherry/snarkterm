# SnarkTerm

SnarkTerm is a planned Linux terminal emulator written in Rust. It aims to provide a fast GPU-accelerated terminal with VT/xterm compatibility, tabs, splits, search, themes, and a sarcastic AI personality layer that comments on command outcomes without interfering with terminal I/O.

The core rule is simple: SnarkTerm must remain a correct, usable terminal even if the personality engine, plugins, database, or local LLM support fail.

## Planned Features

- VT100, VT220, and xterm-compatible terminal behavior.
- GPU rendering with `wgpu`.
- PTY-backed shell sessions using portable Rust crates.
- Tabs, splits, search, configurable themes, and low-latency input.
- Command lifecycle monitoring through shell integration and fallbacks.
- Commentary rendered in a translucent side gutter, never mixed into shell output.
- Personality levels: Professional, Snarky, Unhinged, and British.
- Roast Intensity slider from 0 to 100.
- Local Ollama support for optional dynamic commentary.
- WASM-based plugin system for custom personalities and achievements.
- SQLite-backed statistics and session history.

## Repository Status

This repository currently contains the architecture, crate plan, plugin design, schema, man page, TLDR page, shell integration examples, and implementation roadmap for the project. Runtime implementation will be added incrementally according to the roadmap in `docs/ROADMAP.md`.

## Documentation

- `docs/DESIGN.md`: full technical design with architecture, rendering, PTY, rules, plugins, LLM, security, achievements, and milestone issues.
- `docs/ARCHITECTURE.md`: system architecture, crate layout, rendering, PTY, and personality design.
- `docs/PLUGIN_API.md`: planned plugin model and WASM sandbox contract.
- `docs/SCHEMA.md`: planned SQLite schema.
- `docs/ROADMAP.md`: implementation phases and release milestones.
- `docs/PRIVACY.md`: privacy, local LLM, redaction, and safety expectations.
- `man/snarkterm.1`: useful Unix man page with the appropriate amount of disappointment.
- `tldr/snarkterm.md`: concise usage examples for users with builds still running.
- `examples/shell/`: Bash, Zsh, and Fish OSC 777 shell integration examples.

## Current Skeleton

The Rust workspace is intentionally skeletal but compile-checked. It defines the crate boundaries and shared event types that future implementation work will build on. SnarkTerm is not yet a usable terminal emulator, which is a shame, but so was most software at some point and look how confidently it shipped.

## License

License selection is pending. Apache-2.0 or MIT are both reasonable choices for this project.
