# Security Policy

Report security issues privately once a project contact is published.

Security-sensitive areas include:

- PTY handling.
- Shell integration hooks.
- Clipboard escape sequences.
- Plugin sandboxing.
- Ollama prompt construction and redaction.
- Local command history storage.

SnarkTerm should default to local-only behavior and avoid sending command data to any remote service.
