# SnarkTerm

A GPU-accelerated Linux terminal emulator written in Rust with a sarcastic AI personality.

## Features

- **Native GPU rendering** via `wgpu` with proper monospace font rasterization (`ab_glyph`)
- **Real PTY integration** — launches your actual shell with full terminal support
- **Scrollback buffer** with Page Up/Down navigation
- **Color support** — ANSI 16-color and 24-bit truecolor via SGR sequences
- **Text attributes** — bold, dim, italic, underline, reverse video, strikethrough
- **Keyboard shortcuts** — Ctrl+C/D/Z/L, function keys F1–F12, arrow keys
- **Clipboard** — Ctrl+Shift+C to copy, Ctrl+Shift+V to paste
- **PTY resize** — window resize propagates to the shell process
- **Sarcastic commentary** — judgmental remarks on your command history

## Install

```bash
cargo install --path crates/snarkterm-app
```

Or build from source:

```bash
git clone https://github.com/ghostfrogcherry/snarkterm.git
cd snarkterm
cargo build --release
```

## Usage

```bash
# Launch the native GPU window
snarkterm --window

# Run a command with snarky commentary
snarkterm -c "ls -la"
snarkterm -c "git push --force"
snarkterm -c "asdfghjkl"  # command not found

# Launch a shell in your real terminal
snarkterm
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl+C | Interrupt (SIGINT) |
| Ctrl+D | End of file |
| Ctrl+Z | Suspend (SIGTSTP) |
| Ctrl+L | Clear screen |
| Ctrl+Shift+C | Copy visible text to clipboard |
| Ctrl+Shift+V | Paste from clipboard |
| Page Up/Down | Scroll through history |
| F1–F12 | Function keys |
| Arrow keys | Cursor movement |
| Home/End | Beginning/end of line |

## Snark Examples

```
$ snarkterm -c "rm -rf /"
SnarkTerm: I see we've arrived at the burn down the library to find the bookmark phase.

$ snarkterm -c "git push --force origin main"
SnarkTerm: Force push detected. Somewhere, a future coworker just developed a migraine.

$ snarkterm -c "asdfghjkl"
SnarkTerm: Command not found. Either it doesn't exist, or it's hiding from you.

$ snarkterm -c "echo hello"
SnarkTerm: Exit code 0. A rare and beautiful creature, like a printer that works.
```

## Architecture

```
snarkterm-app          CLI, GPU window renderer, keyboard input
snarkterm-core         Terminal grid, CSI/SGR parser, types
snarkterm-pty          Shell integration markers, PTY abstractions
snarkterm-personality  Canned commentary generation
snarkterm-rules        Danger command detection
snarkterm-plugins      Plugin manifest and permissions (WASM, upcoming)
snarkterm-config       TOML configuration schema
snarkterm-db           SQLite schema for stats/achievements
snarkterm-llm          Ollama client for local LLM integration
snarkterm-render       Render abstractions
snarkterm-ui           UI component types
snarkterm-testkit      Test utilities
```

## License

MIT OR Apache-2.0
