# SnarkTerm Technical Design

SnarkTerm is a serious Linux terminal emulator with one additional subsystem: judgment. It should compete architecturally with modern GPU terminals while keeping the personality layer safely outside terminal I/O, because putting jokes into stdout is how you get bug reports from people who use `awk` recreationally.

## 1. Project Overview

SnarkTerm provides:

- VT100, VT220, and xterm-compatible terminal behavior.
- PTY-backed shell sessions.
- GPU rendering through `wgpu`.
- Tabs, splits, search, scrollback, selection, copy/paste, and themes.
- Configurable keyboard shortcuts.
- A side-panel snark gutter for command commentary.
- Optional local Ollama commentary.
- WASM plugins for custom personalities.
- SQLite stats and achievements.

The personality engine is decorative, like a raccoon in a waistcoat: amusing, occasionally insightful, and absolutely not allowed near the wiring.

## 2. Core Architecture

Communication is event-driven and intentionally one-way where safety matters.

```text
Keyboard/mouse input
  -> input mapper
  -> PTY writer
  -> shell

PTY reader
  -> VT parser
  -> terminal state
  -> render model
  -> wgpu renderer

PTY reader / OSC markers
  -> command monitor
  -> terminal events
  -> rules engine
  -> personality engine / plugins / Ollama
  -> commentary model
  -> snark gutter renderer
```

The shell never receives commentary. The terminal buffer never contains commentary. The user can pipe output to a file without accidentally preserving a joke about their deployment strategy for legal discovery.

## 3. Rust Crate Layout

```text
snarkterm/
  crates/
    snarkterm-app/
    snarkterm-core/
    snarkterm-pty/
    snarkterm-render/
    snarkterm-ui/
    snarkterm-personality/
    snarkterm-rules/
    snarkterm-plugins/
    snarkterm-config/
    snarkterm-db/
    snarkterm-llm/
    snarkterm-testkit/
  assets/
  docs/
  examples/
  plugins/
  man/
  tldr/
```

Crate responsibilities:

- `snarkterm-app`: binary entrypoint, window lifecycle, dependency wiring, top-level event loop.
- `snarkterm-core`: terminal state, grid, scrollback, command event types, shared domain models.
- `snarkterm-pty`: PTY spawning, shell lifecycle, resize handling, shell integration marker parsing.
- `snarkterm-render`: `wgpu` device/surface setup, glyph atlas, draw passes, GPU buffers.
- `snarkterm-ui`: tabs, splits, snark gutter state, settings, stats page, overlays.
- `snarkterm-personality`: profile-specific commentary generation and session memory.
- `snarkterm-rules`: deterministic rule evaluation for danger, repetition, achievements, and command categories.
- `snarkterm-plugins`: WASM plugin loading, manifests, permissions, fuel/time limits.
- `snarkterm-config`: TOML schema, config discovery, validation, defaults.
- `snarkterm-db`: SQLite migrations, stats, sessions, commands, commentary persistence.
- `snarkterm-llm`: Ollama provider, prompt templates, timeout/fallback handling.
- `snarkterm-testkit`: fixtures, VT compatibility harnesses, fake PTY streams, golden render data.

## 4. Rendering Pipeline Using `wgpu`

Window creation uses `winit`. `snarkterm-app` creates a window, passes it to `snarkterm-render`, and drives redraw requests from input, PTY updates, animations, gutter changes, and timers.

Surface setup:

```text
winit window
  -> wgpu instance
  -> surface
  -> adapter selection
  -> device and queue
  -> surface configuration
  -> render pipelines
```

Frame pipeline:

```text
Begin frame
  -> collect dirty panes
  -> shape changed text runs
  -> upload missing glyphs to atlas
  -> update terminal instance buffers
  -> update selection/cursor buffers
  -> update gutter UI buffers
  -> render background pass
  -> render terminal cell pass
  -> render glyph pass
  -> render cursor/selection pass
  -> render UI chrome pass
  -> render snark gutter pass
  -> present
```

Glyph atlas:

- Shape text with `cosmic-text` or equivalent.
- Cache glyphs by font face, size, glyph id, and subpixel bucket.
- Evict least-recently-used glyphs when atlas pressure is high.
- Keep emoji/fallback handling explicit. The terminal should display text, not experience a font-related spiritual event.

Damage tracking:

- PTY parser marks changed rows.
- Scroll operations mark ranges.
- Cursor blink marks cursor cell only.
- Gutter updates do not dirty terminal cells.
- Full redraw is allowed after resize, theme changes, or GPU recovery.

HiDPI:

- Track window scale factor from `winit`.
- Store logical terminal cell size separately from physical pixels.
- Rebuild glyph rasterization when scale changes.

GPU fallback:

- If high-performance adapter selection fails, try low-power adapter.
- If surface creation fails, display a clear startup error.
- A CPU renderer may be a later fallback, but the MVP may fail gracefully with a useful message instead of pretending ANSI art is a graphics backend.

The snark gutter is a UI layer. It is not terminal output. It is not scrollback. It is not copied by terminal selection unless the user explicitly copies gutter text.

## 5. PTY Design

PTY management uses `portable-pty` initially.

Responsibilities:

- Spawn the configured shell.
- Set environment variables.
- Start read and write loops.
- Forward input bytes.
- Emit resize events.
- Track process lifecycle.
- Forward signals where appropriate.
- Surface shell exits without taking the UI down with it.

PTY read loop:

```text
read bytes from PTY
  -> send bytes to VT parser
  -> scan OSC markers
  -> emit shell integration events
```

PTY write loop:

```text
input event
  -> keybinding layer
  -> terminal encoder
  -> PTY writer
```

Terminals do not naturally know what a command is. A terminal sees bytes, escape sequences, cursor movement, and occasionally evidence of human optimism. Command detection therefore uses optional shell integration for correctness and heuristics only as fallback.

## 6. Input Handling

Input stages:

- `winit` keyboard/mouse events.
- Keybinding resolver.
- UI command handler for app shortcuts.
- Terminal input encoder for shell input.
- PTY writer.

Examples:

- `Ctrl+Shift+T` opens a tab and does not go to the shell.
- `Ctrl+C` goes to the shell unless captured by a focused UI overlay.
- Bracketed paste wraps pasted text when enabled.
- Mouse events are encoded for terminal apps when mouse reporting is enabled.

Clipboard handling must never auto-execute pasted commands. The user may ruin things manually, as tradition demands.

## 7. Scrollback and Terminal State

Terminal state contains:

- Primary grid.
- Alternate screen grid.
- Cursor state.
- Tab stops.
- DEC/xterm modes.
- Style attributes.
- Scroll regions.
- Scrollback ring.
- Search index.

Scrollback should be a bounded ring of logical lines. Reflow on resize is desirable after basic correctness lands. Alternate screen content should not pollute normal scrollback unless explicitly configured.

Search indexing runs off the render hot path. Nobody wants typing latency because the terminal is lovingly indexing the word `node_modules` for the 8,000th time.

## 8. Tabs and Splits Model

Tabs own layout trees. Leaves are panes. Panes own PTY sessions.

```rust
pub enum SplitNode {
    Pane { session_id: SessionId },
    Horizontal { ratio: f32, children: Vec<SplitNode> },
    Vertical { ratio: f32, children: Vec<SplitNode> },
}
```

Resizing a split recalculates pane rectangles and emits PTY resize events. Closing a pane terminates or detaches its session according to user settings.

## 9. Command Monitoring

SnarkTerm prefers shell integration markers using private OSC 777 sequences.

Marker format:

```text
OSC 777;snarkterm;event=prompt_start BEL
OSC 777;snarkterm;event=command_start;id=...;cwd=...;command=... BEL
OSC 777;snarkterm;event=command_end;id=...;status=1;duration_ms=842;cwd=... BEL
```

The VT parser consumes OSC 777 markers and does not render them. Unknown OSC 777 data is ignored. Malformed markers are logged and discarded with the quiet dignity of a senior engineer reading a Jira ticket.

Fallback detection:

- Track typed input until Enter.
- Detect prompt-ish output heuristically.
- Duration starts on Enter.
- Exit status may be unknown.

Limitations:

- Shell aliases, functions, traps, multiline commands, TUIs, remote shells, and nested shells can confuse detection.
- `ssh` sessions are separate worlds unless remote integration is installed.
- `sudo` can alter environment and shell behavior.
- This is engineering, not clairvoyance.

## 10. Personality Engine

Input:

```rust
pub enum TerminalEvent {
    CommandStarted(CommandInfo),
    CommandCompleted(CommandResult),
    CommandFailed(CommandResult),
    DangerousCommandDetected(DangerousCommand),
    LongRunningCommand(LongRunningInfo),
    RepeatedMistake(MistakeInfo),
    SessionMilestone(SessionStats),
}
```

Output:

```rust
pub struct Commentary {
    pub text: String,
    pub severity: CommentarySeverity,
    pub personality: PersonalityProfile,
    pub created_at: DateTime<Utc>,
    pub ttl: Option<Duration>,
    pub related_command_id: Option<CommandId>,
}
```

Profiles:

- Professional: `Command completed successfully. Nobody was harmed, statistically speaking.`
- Snarky: `Exit code 0. A rare and beautiful creature, like a printer that works.`
- Unhinged: `The command failed again. At this point the bug has squatters' rights.`
- British: `A bold command. Not a correct one, naturally, but bold.`

The engine uses deterministic rules first, then plugins, then optional Ollama. Canned fallback commentary always exists because dependency on an LLM for jokes is how civilization ends, slowly, with a spinner.

## 11. Plugin API

Plugins may be WASM first, dynamic libraries later if someone asks for more ways to debug undefined behavior.

Manifest:

```toml
[plugin]
name = "corporate-disappointment"
version = "0.1.0"
api_version = "1"

[permissions]
read_command_metadata = true
read_raw_output = false
network = false
filesystem = false
```

Plugin execution:

- Host sends serialized event.
- Plugin returns zero or more commentary messages.
- Host validates size, severity, TTL, and content.
- Host applies timeout/fuel limits.
- Host disables plugin after repeated failures.

Plugins cannot write to PTY input. Plugins cannot mutate command text. Plugins cannot become an AI agent. SnarkTerm judges commands; it does not grab the steering wheel.

## 12. Ollama Integration

Ollama is optional and local-only by default.

Pipeline:

```text
terminal event
  -> redaction
  -> prompt template
  -> timeout-bound Ollama request
  -> response size limit
  -> safety filter
  -> gutter commentary
```

Rules:

- Timeout default: 750 ms.
- Max response: 80 tokens.
- No raw output unless explicitly enabled.
- Secrets redacted before prompt construction.
- Failure falls back to canned text or silence.
- Rendering and PTY loops never wait on Ollama.

Sample prompt:

```text
You are SnarkTerm, a dry, non-hostile terminal commentator.
Profile: British
Roast intensity: 65
Command: git push --force
Exit status: 0
Duration: 1220ms
Rules matched: force-push
Return one short sentence. No advice unless necessary. No markdown.
```

## 13. I/O Separation Rules

- Commentary never writes to PTY.
- Commentary never enters terminal scrollback.
- Commentary is copied only through gutter UI.
- OSC markers are consumed before rendering.
- Unknown markers never execute behavior.
- Plugins and LLM providers receive copies of metadata, not control handles.
- Full-screen TUIs may suppress gutter popups by default.

## 14. Crash Isolation

Failure domains:

- PTY session crash closes or marks that pane.
- Renderer failure attempts surface/device recovery.
- Personality panic disables personality for the session.
- Plugin failure disables that plugin.
- DB failure switches to in-memory stats.
- Ollama failure uses fallback or silence.

The main terminal path is the protected path. If the joke system crashes, the shell keeps working. Imagine that: accountability.

## 15. Configuration Format

SnarkTerm uses TOML at `~/.config/snarkterm/config.toml`.

```toml
[ui]
theme = "neon-midnight"
font_family = "JetBrainsMono Nerd Font"
font_size = 13.0
snark_gutter = true
gutter_width = 360
transparency = 0.82

[personality]
profile = "snarky"
roast_intensity = 65
comment_on_success = true
comment_on_failure = true
comment_on_danger = true
comment_on_long_running = true
work_mode = false

[llm]
enabled = false
provider = "ollama"
model = "llama3.1"
timeout_ms = 750
redact_secrets = true

[stats]
enabled = true
store_command_text = true
store_raw_output = false

[privacy]
telemetry = false
local_only = true

[keybindings]
split_horizontal = "Ctrl+Shift+H"
split_vertical = "Ctrl+Shift+V"
new_tab = "Ctrl+Shift+T"
close_tab = "Ctrl+Shift+W"
toggle_snark = "Ctrl+Shift+S"
```

Work mode means: be useful, be brief, and stop acting like every failed `kubectl` command is dinner theater.

## 16. Database Schema

The full schema lives in `docs/schema.sql`. It tracks sessions, commands, results, commentary, rule matches, achievements, user stats, settings, and plugin events.

Raw output is not stored by default. Local stats are not telemetry. Telemetry defaults off, because the product is judgmental, not creepy.

## 17. Statistics Tracking

Stats page tracks:

- Failed commands.
- `sudo` count.
- Force pushes.
- `chmod 777` count.
- Restart-based fixes.
- Longest build.
- Most failed command.
- Most restarted service.
- Most dangerous command attempted.
- Total build wait time.
- Commands that worked after “just changing one tiny thing.”

Example label:

```text
Restart-based remediation events: 14
SnarkTerm: Infrastructure solved by rebooting. Humanity's oldest spell.
```

## 18. Man Page

The man page lives at `man/snarkterm.1`.

## 19. TLDR Page

The TLDR page lives at `tldr/snarkterm.md`.

## 20. Implementation Roadmap

See `docs/ROADMAP.md` for phases 0 through 9.

## 21. Testing Strategy

Testing layers:

- Unit tests for parser state, rules, config, redaction, and session stats.
- Golden tests for OSC marker parsing.
- PTY integration tests using fake shells and scripted output.
- VT compatibility smoke tests with `vttest`.
- UI layout tests for tabs, splits, gutter states, and resizing.
- Render golden tests where practical.
- Plugin sandbox tests for timeouts, memory limits, and denied permissions.
- LLM tests using a fake Ollama server.
- Performance tests for startup, throughput, scrollback, and frame pacing.

Compatibility targets:

- Bash, Zsh, Fish.
- `tmux`, `vim`, `neovim`, `less`, `htop`, `fzf`.
- `ssh` basic behavior.
- Common build tools: `cargo`, `npm`, `make`, `cmake`.

## 22. Security Considerations

SnarkTerm must not:

- Execute commentary.
- Rewrite commands.
- Auto-correct commands.
- Inject shell input.
- Send command data remotely by default.
- Give plugins raw output unless allowed.
- Trust OSC payloads without validation.

Protections:

- Local-only defaults.
- Explicit consent for LLM context sharing.
- Secret redaction.
- Sandboxed plugins.
- Capability manifests.
- OSC length limits.
- Strict OSC grammar.
- Clipboard paste confirmation for dangerous multi-line paste, if enabled.
- No “AI agent” behavior.

SnarkTerm may judge commands, but it must never change them. The user is allowed to ruin their system manually, as tradition demands.

## 23. Example Module Skeletons

Rule trait:

```rust
pub trait Rule {
    fn id(&self) -> &'static str;
    fn evaluate(&self, event: &TerminalEvent, session: &SessionContext) -> Vec<RuleMatch>;
}
```

Danger rule:

```rust
pub struct CurlPipeShellRule;

impl Rule for CurlPipeShellRule {
    fn id(&self) -> &'static str { "curl-pipe-shell" }

    fn evaluate(&self, event: &TerminalEvent, _: &SessionContext) -> Vec<RuleMatch> {
        let TerminalEvent::CommandStarted(info) = event else { return vec![]; };
        let c = info.command.as_str();
        if (c.contains("curl") || c.contains("wget")) && (c.contains("| sh") || c.contains("| bash")) {
            return vec![RuleMatch::danger(self.id(), "A stranger's shell script, piped directly into trust. Inspiring.")];
        }
        vec![]
    }
}
```

Commentary creation:

```rust
Commentary {
    text: "Security model replaced with vibes. Very modern.".into(),
    severity: CommentarySeverity::Danger,
    personality: PersonalityProfile::Snarky,
    created_at: Utc::now(),
    ttl: Some(Duration::from_secs(12)),
    related_command_id: Some(command_id),
}
```

## 24. Suggested GitHub Repo Structure

```text
.github/
  workflows/ci.yml
  ISSUE_TEMPLATE/
assets/
  themes/
crates/
docs/
examples/
man/
plugins/
tldr/
Cargo.toml
README.md
CONTRIBUTING.md
SECURITY.md
LICENSE-MIT
LICENSE-APACHE
```

## 25. First Milestone Issue List

See `docs/MILESTONE_0_ISSUES.md` for the issue-ready checklist.

## Achievement Catalog

| ID | Trigger | Display Text | Commentary |
| --- | --- | --- | --- |
| `friday-deploy` | deploy/push Friday afternoon | Friday Deploy | Deploying near the weekend. Bold scheduling for a person with plans. |
| `production-archaeologist` | repeated prod inspection commands | Production Archaeologist | You have discovered another ancient service with no owner. Congratulations, sort of. |
| `chmod-777-enjoyer` | `chmod 777` | chmod 777 Enjoyer | Permissions are hard, so you made them imaginary. |
| `works-on-my-machine` | local success after remote/deploy failure | Works On My Machine | The sacred phrase has been invoked. Reality may now split. |
| `sudo-sommelier` | high sudo frequency | Sudo Sommelier | A refined palate for privilege escalation. Notes of panic and oak. |
| `dependency-goblin` | package manager churn | Dependency Goblin | Installing the internet again. Dusty work, but someone must summon it. |
| `restartomancer` | repeated restart fixes | Restartomancer | Infrastructure solved by rebooting. Humanity's oldest spell. |
| `force-push-romantic` | force push | Force Push Romantic | Rewriting history because the present became inconvenient. |
| `yaml-cartographer` | extensive YAML edits | YAML Cartographer | Mapping whitespace in a land without mercy. |
| `build-still-running` | very long build | The Build Is Still Running | This build has acquired legal residency. |
| `curl-pipe-philosopher` | `curl | sh` or `wget | bash` | Curl Pipe Philosopher | Trust, but do not verify. Inspirationally backwards. |
| `incident-report-speedrun` | dangerous prod command | Incident Report Speedrun | A personal best in creating follow-up meetings. |
| `stack-overflow-archaeologist` | SO paste heuristic | Stack Overflow Archaeologist | Excavating answers from 2014. The accepted one, naturally, is deprecated. |
| `npm-install-survivor` | huge npm install | npm Install Survivor | You installed the internet and lived to regret the lockfile. |
| `kubernetes-roulette` | risky kubectl prod command | Kubernetes Roulette | One context, one command, and several teams suddenly alert. |
