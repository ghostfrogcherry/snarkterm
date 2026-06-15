# Roadmap

## Phase 1: Terminal Foundation

- Create Rust workspace and crate structure.
- Open a Linux window with `winit`.
- Initialize `wgpu`.
- Spawn shell through `portable-pty`.
- Read and write PTY data.
- Parse ANSI and VT sequences.
- Render a basic terminal grid.
- Implement keyboard input.
- Implement resize handling.

Exit criteria:

- Shell commands run interactively.
- `vim`, `less`, `top`, and `cargo build` are usable.
- Basic terminal correctness works.

## Phase 2: Rendering Quality

- Add glyph atlas.
- Add font fallback and shaping.
- Add 24-bit color support.
- Add cursor styles.
- Add selection.
- Add scrollback.
- Add search.
- Add theme loading.

Exit criteria:

- Comfortable daily terminal usage.
- Good performance on large output.
- Low input latency.

## Phase 3: Tabs, Splits, and UI

- Add layout tree.
- Add tabs.
- Add horizontal and vertical splits.
- Add command palette.
- Add search overlay.
- Add config-backed keybindings.
- Add cyberpunk default theme.

Exit criteria:

- Multi-pane workflows work reliably.
- Layout changes do not disrupt PTY sessions.

## Phase 4: Command Monitoring

- Add shell integration OSC protocol.
- Add Bash, Zsh, and Fish hooks.
- Track command start, finish, duration, and exit code.
- Implement fallback heuristic command tracking.
- Detect `sudo`, force pushes, restarts, and dangerous commands.

Exit criteria:

- Command events are reliable for common shells.
- Stats are captured without corrupting terminal output.

## Phase 5: Personality Engine

- Implement deterministic rules engine.
- Add personality levels.
- Add roast intensity.
- Add repeated mistake memory.
- Add long-running command commentary.
- Render commentary gutter.
- Add crash isolation.

Exit criteria:

- Commentary appears in the side panel.
- Terminal remains functional if personality is disabled or crashes.

## Phase 6: Persistence and Stats

- Add SQLite database.
- Store sessions, commands, commentary, and stats.
- Add statistics page.
- Add achievements.
- Add end-of-day report generator.

Exit criteria:

- Stats survive restart.
- Achievements unlock reliably.
- User can clear history.

## Phase 7: Ollama Support

- Add local Ollama client.
- Add timeout and fallback behavior.
- Add prompt templates.
- Add redaction.
- Add config controls.
- Add opt-in command output context.

Exit criteria:

- Dynamic commentary works locally.
- No rendering or PTY blocking.
- Deterministic fallback is always available.

## Phase 8: Plugin System

- Define plugin API.
- Implement WASM plugin loader.
- Add capability model.
- Add example personality plugin.
- Add example achievement plugin.
- Add plugin documentation.

Exit criteria:

- Third-party plugins can generate commentary safely.
- Bad plugins cannot crash the terminal.

## First Public Alpha

Target version: `0.1.0-alpha`

Codename: `It Probably Works On Your Machine`

Expected scope:

- Linux-only build.
- Single window.
- Tabs and splits.
- Functional PTY terminal.
- GPU text rendering.
- Config file.
- Themes.
- Commentary gutter.
- Deterministic snark engine.
- Basic stats.
- Optional Ollama support.
- Plugin API marked unstable.
