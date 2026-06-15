# Privacy and Safety

SnarkTerm observes command metadata to provide commentary and statistics. Privacy defaults should be conservative.

## Defaults

- No cloud LLM calls.
- Ollama integration disabled by default.
- Command output is not stored by default.
- Command output is not sent to Ollama by default.
- Command history is local-only.
- Incognito mode disables stats and session memory.
- Users can clear all local history.

## Redaction

Before any command metadata is sent to Ollama or plugins, SnarkTerm should redact common secret patterns.

Examples:

- `--password ...`
- `--token ...`
- `AWS_SECRET_ACCESS_KEY`
- `GITHUB_TOKEN`
- `Authorization:`
- `Bearer ...`
- SSH keys.
- `.env` values.

## Dangerous Commands

Dangerous command detection is advisory by default. SnarkTerm may display warnings, but it should not block execution unless the user explicitly enables a command guard mode.

## Plugin Safety

- Plugins are sandboxed.
- Plugins cannot write to PTY input.
- Plugins cannot modify terminal output.
- Plugins cannot access network or filesystem APIs without explicit capabilities.
- Plugin failures disable the plugin, not the terminal.
