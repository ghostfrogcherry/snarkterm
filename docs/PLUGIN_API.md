# Plugin API

SnarkTerm plugins are planned as WASM modules with a small capability-based API. The goal is to allow custom personalities, rules, achievements, and themes without giving third-party code direct access to terminal I/O.

## Plugin Types

- Personality plugins generate commentary for command events.
- Rule plugins classify commands or produce deterministic responses.
- Achievement plugins unlock badges from command and stats events.
- Theme plugins provide colors and UI presentation metadata.

## Manifest

```toml
[plugin]
id = "com.example.deadpan"
name = "Deadpan Mode"
version = "0.1.0"
api_version = "1"

[capabilities]
observe_commands = true
emit_commentary = true
read_stats = true
network = false
filesystem = false
```

## Event Model

```rust
pub enum PluginEvent {
    CommandStarted(CommandStarted),
    CommandFinished(CommandFinished),
    LongRunning(LongRunningCommand),
    StatsUpdated(StatsSnapshot),
}
```

## Personality Plugin Shape

```rust
pub trait PersonalityPlugin {
    fn metadata(&self) -> PluginMetadata;

    fn on_command_started(
        &mut self,
        event: CommandStarted,
        ctx: PluginContext,
    ) -> PluginResult<Vec<Commentary>>;

    fn on_command_finished(
        &mut self,
        event: CommandFinished,
        ctx: PluginContext,
    ) -> PluginResult<Vec<Commentary>>;

    fn on_long_running(
        &mut self,
        event: LongRunningCommand,
        ctx: PluginContext,
    ) -> PluginResult<Vec<Commentary>>;
}
```

## Commentary

```rust
pub struct Commentary {
    pub severity: CommentarySeverity,
    pub text: String,
    pub ttl_ms: Option<u64>,
    pub command_id: Option<CommandId>,
}

pub enum CommentarySeverity {
    Info,
    Success,
    Warning,
    Danger,
    Achievement,
}
```

## Safety Model

- Plugins run in a WASM sandbox.
- Plugins receive fuel and memory limits.
- Network and filesystem access are disabled by default.
- Plugin output is filtered by the same safety layer used for Ollama output.
- Plugin crashes disable the plugin, not the terminal.
- Plugins cannot write to PTY input or terminal output.

## ABI Stability

The plugin ABI should be marked unstable until the first beta. Public plugin examples should be versioned against explicit API versions.
