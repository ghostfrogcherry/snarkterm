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

SnarkTerm now has a usable first cut: a PTY-backed terminal binary named `snarkterm` that can launch your shell or run a command through your shell. The full GPU window, tabs, splits, side gutter, and renderer are still under active implementation.

Current reality, because marketing is how software lies to itself:

- Usable: PTY-backed shell launch in the current terminal.
- Usable: `snarkterm -c <COMMAND>` command mode with commentary.
- Usable: basic CLI help/version behavior.
- Usable preview: native `winit`/`wgpu` window with GPU surface initialization via `--window`.
- Planned: terminal grid rendering inside the native GPU window.
- Planned: real snark gutter instead of command-mode stderr commentary.
- Planned: shell integration, rules, stats, Ollama, plugins, tabs, and splits.

## Install And Run

Build the binary:

```sh
cargo build --release -p snarkterm-app --bin snarkterm
```

Run it from the repo:

```sh
cargo run -p snarkterm-app --bin snarkterm
```

Run a single command with commentary:

```sh
cargo run -p snarkterm-app --bin snarkterm -- -c 'printf hello'
```

Launch the native GPU window preview:

```sh
cargo run -p snarkterm-app --bin snarkterm -- --window
```

Install locally with Cargo:

```sh
cargo install --path crates/snarkterm-app
```

Then run:

```sh
snarkterm
```

If you want fewer comments from the rectangle with opinions:

```sh
snarkterm -c 'false' --no-commentary
```

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

## Current Product State

The Rust workspace defines the crate boundaries and shared event types that future implementation work will build on. The `snarkterm` binary is intentionally minimal, but it is no longer a decorative README. It opens a real PTY, spawns your shell, forwards bytes, restores raw mode on exit, and can launch a native GPU window preview.

Known limitations:

- GPU window exists, but terminal text rendering is not wired into it yet.
- No scrollback owned by SnarkTerm yet.
- No tabs/splits yet.
- No side gutter yet.
- Interactive mode depends on the host terminal emulator for display.
- Resize handling is still basic.
- Commentary in command mode prints to stderr until the real UI gutter exists.

## License

License selection is pending. Apache-2.0 or MIT are both reasonable choices for this project.
