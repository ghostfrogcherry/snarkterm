# Architecture

SnarkTerm is split into a strict terminal core and an optional personality layer. The terminal core owns PTY I/O, VT parsing, grid state, input encoding, and rendering. The personality layer only observes command events and emits commentary to a UI gutter.

## Core Principle

Personality features must never affect terminal correctness.

Hard rules:

- Commentary never enters PTY input.
- Commentary never enters PTY output.
- Commentary is rendered only in a dedicated gutter or overlay model.
- Personality, Ollama, plugins, and persistence must not block the PTY or render hot paths.
- If personality systems fail, the terminal continues functioning.

## High-Level Components

```text
snarkterm
├── App Runtime
│   ├── Window and event loop
│   ├── Config loader
│   ├── Layout manager
│   └── Crash isolation
├── Terminal Core
│   ├── PTY manager
│   ├── Shell sessions
│   ├── VT parser
│   ├── Terminal grid
│   ├── Scrollback
│   ├── Search index
│   └── Input encoder
├── Renderer
│   ├── wgpu backend
│   ├── Glyph atlas
│   ├── Text renderer
│   ├── Cursor renderer
│   ├── Selection renderer
│   ├── Split and tab UI renderer
│   └── Commentary gutter renderer
├── Personality System
│   ├── Command detector
│   ├── Rules engine
│   ├── Session memory
│   ├── Roast generator
│   ├── Ollama client
│   ├── Plugin runtime
│   └── Safety filter
├── Persistence
│   ├── SQLite DB
│   ├── Stats store
│   ├── Achievement store
│   └── Session journal
└── Plugin API
    ├── Personality plugins
    ├── Rule plugins
    ├── Achievement plugins
    └── Theme plugins
```

## Rust Workspace Layout

```text
crates/
├── snarkterm-app/
│   ├── main.rs
│   ├── app.rs
│   ├── window.rs
│   ├── layout.rs
│   └── commands.rs
├── snarkterm-core/
│   ├── terminal.rs
│   ├── grid.rs
│   ├── cell.rs
│   ├── scrollback.rs
│   ├── selection.rs
│   ├── search.rs
│   └── input.rs
├── snarkterm-pty/
│   ├── pty.rs
│   ├── session.rs
│   ├── shell.rs
│   ├── monitor.rs
│   └── command_detection.rs
├── snarkterm-vt/
│   ├── parser.rs
│   ├── ansi.rs
│   ├── osc.rs
│   ├── csi.rs
│   └── xterm.rs
├── snarkterm-render/
│   ├── renderer.rs
│   ├── pipeline.rs
│   ├── glyph_atlas.rs
│   ├── text.rs
│   ├── cursor.rs
│   ├── gutter.rs
│   ├── themes.rs
│   └── shaders/
├── snarkterm-personality/
│   ├── engine.rs
│   ├── rules.rs
│   ├── profiles.rs
│   ├── commentary.rs
│   ├── memory.rs
│   ├── dangerous.rs
│   ├── ollama.rs
│   └── safety.rs
├── snarkterm-plugin-api/
│   ├── lib.rs
│   ├── event.rs
│   ├── personality.rs
│   ├── rules.rs
│   └── abi.rs
├── snarkterm-plugins/
│   ├── loader.rs
│   ├── wasm.rs
│   └── registry.rs
├── snarkterm-db/
│   ├── db.rs
│   ├── migrations.rs
│   ├── stats.rs
│   ├── achievements.rs
│   └── sessions.rs
└── snarkterm-config/
    ├── config.rs
    ├── theme.rs
    ├── keybindings.rs
    └── profiles.rs
```

## Recommended Dependencies

```toml
[dependencies]
wgpu = "0.20"
winit = "0.30"
cosmic-text = "0.12"
portable-pty = "0.8"
vte = "0.13"
tokio = { version = "1", features = ["full"] }
crossbeam-channel = "0.5"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json"] }
wasmtime = "22"
tracing = "0.1"
tracing-subscriber = "0.3"
```

Mature terminal-core reuse should be considered before implementing all emulation behavior from scratch. Candidates include `alacritty_terminal` or `wezterm-term`, subject to licensing and architectural fit.

## Runtime Model

```text
Main Thread
├── winit event loop
├── keyboard and mouse input
├── window lifecycle
├── layout updates
└── render scheduling

PTY Thread per Session
├── shell process
├── PTY read loop
├── PTY write channel
└── resize handling

Terminal Worker
├── VT parsing
├── grid mutation
├── scrollback updates
└── command lifecycle events

Personality Worker
├── observes command events
├── applies local rules
├── optionally calls Ollama
├── updates session memory
└── emits commentary events
```

Dataflow:

```text
PTY output -> VT parser -> terminal grid -> renderer
PTY output -> command monitor -> personality engine -> gutter model -> renderer
```

## PTY Design

Each pane owns one PTY-backed shell session.

```rust
pub struct PtySession {
    pub id: SessionId,
    pub shell: ShellConfig,
    pub size: PtySize,
    pub writer: PtyWriter,
    pub reader_handle: JoinHandle<()>,
    pub command_monitor: CommandMonitor,
}
```

Responsibilities:

- Spawn the configured login shell.
- Read raw bytes from the PTY.
- Send bytes to the terminal parser.
- Send an observation copy to command monitoring.
- Handle resize events.
- Write encoded keyboard and mouse input.
- Kill or restart shell sessions on user request.

Input flow:

```text
User input -> keybinding resolver -> terminal input encoder -> PTY writer -> shell
```

Output flow:

```text
PTY reader -> VT parser -> terminal grid -> renderer
```

## Command Monitoring

Command monitoring should be shell-integrated when possible and heuristic when not.

Preferred approach:

- Bash, Zsh, and Fish hooks emit private OSC events.
- Pre-exec emits command text, working directory, and timestamp.
- Pre-command prompt hook emits exit status and duration.
- SnarkTerm listens for private OSC `777` events.

Fallback approach:

- Track user-entered command lines when Enter is pressed.
- Infer prompt boundaries from terminal output.
- Treat exit status as unknown when shell integration is unavailable.

Example Bash integration concept:

```bash
_snarkterm_preexec() {
  printf '\033]777;preexec;%s\007' "$BASH_COMMAND"
}

_snarkterm_precmd() {
  printf '\033]777;precmd;%s\007' "$?"
}

trap '_snarkterm_preexec' DEBUG
PROMPT_COMMAND="_snarkterm_precmd;$PROMPT_COMMAND"
```

## VT/xterm Compatibility

Initial parser work can use `vte` unless a mature terminal core is adopted.

Supported behavior should include:

- C0 and C1 controls.
- CSI sequences.
- OSC sequences.
- Alternate screen.
- Bracketed paste.
- Mouse reporting modes.
- SGR styling.
- 24-bit color.
- Cursor shapes.
- Scroll regions.
- DEC private modes.
- xterm clipboard sequences gated behind config.
- Private SnarkTerm shell integration OSC.

Compatibility should be validated with:

- `vttest`
- `ncurses`
- `tmux`
- `vim` and `neovim`
- `less`
- `htop`
- `fzf`
- `git`
- `cargo`
- `ssh`

## Rendering Pipeline

Rendering uses `wgpu` and keeps terminal text dominant over UI decoration.

```text
Frame begin
├── acquire surface texture
├── update uniforms
├── update glyph atlas
├── build terminal text instances
├── build cursor instances
├── build selection instances
├── build UI chrome instances
├── build gutter instances
├── encode render passes
└── present
```

Render passes:

- Background: window, panes, split backgrounds, optional subtle theme effects.
- Terminal cells: cell backgrounds, selection, search highlights, cursor background.
- Text: glyph atlas sampling, shaping, styles, subpixel positioning.
- UI: tabs, split borders, title bars, search box, command palette.
- Commentary gutter: translucent panel, cards, severity markers, slider, statistics entry points.

Text pipeline:

```text
Terminal grid -> style/color/font runs -> shape glyphs -> upload missing glyphs -> draw instanced quads
```

`cosmic-text` is the preferred starting point for shaping and fallback.

## UI Design

The default visual language is minimalist cyberpunk: dark base colors, restrained neon accents, subtle translucency, and no noisy decoration that competes with terminal content.

Default theme concept:

```toml
[theme]
name = "Neon Restraint"
background = "#080A0F"
foreground = "#D8DEE9"
accent = "#00F5D4"
danger = "#FF3B6B"
warning = "#FFB86B"
success = "#8AFF80"
panel_background = "#10131CCC"
panel_border = "#00F5D455"
cursor = "#00F5D4"
selection = "#263248"
```

Core UI elements:

- Tab strip.
- Split borders.
- Search overlay.
- Command palette.
- Commentary gutter.
- Roast Intensity slider.
- Statistics page.
- Personality profile selector.

## Personality Engine

The engine consumes command events and emits commentary events.

```rust
pub enum CommandEvent {
    Started(CommandStarted),
    Finished(CommandFinished),
    OutputObserved(OutputObserved),
    LongRunning(LongRunningCommand),
    DangerousCommandDetected(DangerousCommand),
}
```

```rust
pub struct CommandFinished {
    pub session_id: SessionId,
    pub command: String,
    pub cwd: PathBuf,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}
```

Pipeline:

```text
Command event
  -> normalize command
  -> classify command
  -> update stats
  -> update session memory
  -> apply safety rules
  -> apply personality profile
  -> maybe call Ollama
  -> post-process commentary
  -> emit gutter message
```

Personality levels:

- Professional: dry, useful, low sarcasm.
- Snarky: default, witty but non-hostile.
- Unhinged: theatrical and chaotic, still non-abusive.
- British: understated disappointment and polite judgment.

Roast intensity affects frequency, sharpness, repeated-mistake references, Ollama prompt style, and whether successful commands get commentary.

## Rules Engine

Rules are deterministic and run before LLM generation.

```rust
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn matches(&self, event: &CommandEvent, ctx: &RuleContext) -> bool;
    fn produce(&self, event: &CommandEvent, ctx: &RuleContext) -> RuleResult;
}
```

Initial rule categories:

- Success.
- Failure.
- Dangerous command.
- Long-running process.
- Repeated mistake.
- Sudo usage.
- Force push.
- `chmod 777`.
- Restart fix.
- Build command.
- Deployment command.
- Stack Overflow paste heuristic.
- Random blog command heuristic.

Dangerous patterns include:

- `rm -rf /`
- `rm -rf ~`
- `mkfs.*`
- `dd if=...`
- `chmod -R 777`
- `sudo systemctl stop`
- `git push --force`
- `kubectl delete`
- `terraform destroy`
- `docker system prune -a`

## Long-Running Commands

The engine tracks active commands and emits throttled commentary.

Example cadence:

- 30 seconds: `Still running. Optimism remains technically available.`
- 2 minutes: `This has become less of a command and more of a relationship.`
- 5 minutes: `Compiling dependencies from what appears to be the Bronze Age.`
- 15 minutes: `At this point, the fans are emotionally involved.`

Commentary should be suppressed or reduced for full-screen TUIs by default.

## Session Memory

Session memory is short-lived by default.

```rust
pub struct SessionMemory {
    pub failed_commands: HashMap<String, usize>,
    pub sudo_count: usize,
    pub force_push_count: usize,
    pub restart_fix_count: usize,
    pub dangerous_attempts: Vec<DangerousAttempt>,
}
```

Tracked examples:

- Repeated failed commands.
- Repeated typo patterns.
- Frequent `sudo` usage.
- Restart-based fixes.
- Force pushes.
- Common dangerous operations.
- Build duration baselines.

## Ollama Integration

Ollama is optional, local-only, timeout-bound, and never required for commentary.

Default endpoint:

```text
http://localhost:11434/api/generate
```

Default config:

```toml
[personality.ollama]
enabled = false
model = "llama3.1:8b"
timeout_ms = 1200
max_tokens = 80
send_command_output = false
```

Prompt style:

```text
You are SnarkTerm, a sarcastic but non-hostile terminal assistant.
Personality: British
Roast intensity: 65
Command: cargo build
Exit code: 0
Duration: 74s
Context: Rust project, long build
Return one short sentence. Do not include shell output. Do not give instructions unless asked.
```

## Configuration Example

```toml
[terminal]
shell = "/bin/bash"
font_family = "JetBrains Mono"
font_size = 13.5
scrollback_lines = 100000
startup_mode = "fast"

[window]
opacity = 0.96
decorations = true
default_columns = 120
default_rows = 36

[renderer]
backend = "wgpu"
vsync = true
cursor_blink = true

[personality]
enabled = true
level = "snarky"
roast_intensity = 60
commentary_frequency = "moderate"
dangerous_command_warnings = true
remember_session_mistakes = true

[personality.ollama]
enabled = false
model = "llama3.1:8b"
timeout_ms = 1200
send_command_output = false

[gutter]
enabled = true
position = "right"
width = 360
opacity = 0.78

[stats]
enabled = true
persist_history = true

[plugins]
enabled = true
directory = "~/.config/snarkterm/plugins"
```
