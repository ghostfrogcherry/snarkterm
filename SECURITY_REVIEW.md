# SnarkTerm Security Review

> See also: `SECURITY.md` (security policy)

## Security-Sensitive Areas

### 1. PTY Handling (HIGH)

- **Risk:** PTY pipe carries all keyboard input and shell output. A vulnerability could leak keystrokes (passwords, keys) or inject malicious input into the shell.
- **Mitigations planned:** PTY thread isolation, no commentary injection into PTY streams, strict separation of read/write channels.
- **TODO:** Audit PTY read loop for buffer overflows; validate credential/input boundaries.

### 2. Shell Integration Hooks (MEDIUM)

- **Risk:** Shell integration OSC sequences (OSC 777) are parsed from PTY output. Malformed sequences could cause parser crashes or injection.
- **Mitigations planned:** Use `vte` parser; validate OSC payload length and structure; reject unrecognized private OSCs.
- **TODO:** Fuzz the OSC parser; validate shell hook scripts don't leak sensitive data.

### 3. Clipboard Escape Sequences (MEDIUM)

- **Risk:** xterm-style clipboard OSC sequences could allow websites or programs to read/write clipboard without user consent.
- **Mitigations planned:** Gate clipboard sequences behind config flag; require user interaction (Ctrl+Shift+C/V) to be primary clipboard path.
- **TODO:** Default clipboard OSC to disabled; audit clipboard flow.

### 4. Plugin Sandboxing (HIGH)

- **Risk:** WASM plugins could access filesystem, network, or PTY if sandbox is misconfigured.
- **Mitigations planned:** Use `wasmtime` with capability model; restrict to personality API only; no filesystem/network by default.
- **TODO:** Design capability permission model; audit wasmtime integration for sandbox escapes.

### 5. Ollama Prompt Construction (MEDIUM)

- **Risk:** Commands and output sent to local Ollama could contain sensitive data (passwords, tokens, keys). Prompt injection could leak data or produce harmful output.
- **Mitigations planned:** Redact sensitive patterns (e.g., `--password`, `AWS_SECRET_KEY`); optional command output context (disabled by default); configurable timeout.
- **TODO:** Build redaction rules; review prompt templates for injection vectors; ensure no data sent to remote endpoints.

### 6. Local Command History Storage (LOW)

- **Risk:** SQLite database on disk stores command history indefinitely, creating forensic trail.
- **Mitigations planned:** Offer clear-history command; encrypt on disk if possible; respect user deletion.
- **TODO:** Add schema review; add database encryption consideration; add vacuum and cleanup routines.

### 7. GPU / wgpu Rendering (LOW-MEDIUM)

- **Risk:** Shader compilation and GPU memory could be exploited via crafted glyph atlases or textures.
- **Mitigations planned:** Validate glyph sizes and texture dimensions; no network access in shaders; bounded memory allocation.
- **TODO:** Review wgpu buffer/ texture creation for upper bounds; validate font file parsing (ab_glyph) for security.

## Dependency Concerns

| Dependency | Concern | Status |
|------------|---------|--------|
| `portable-pty` | PTY management | Review for fork/exec safety |
| `rusqlite` (bundled) | SQLite C library | Keep updated; bundled means no system SQLite CVEs |
| `reqwest` | HTTP client (Ollama) | Only for localhost by default; disable by default |
| `wgpu` | GPU compute | Audited by Mozilla/community; safe defaults |
| `wasmtime` | WASM sandbox | Strong sandbox; verify capability model |

## Recommended Security Practices

1. Run `cargo deny` to check license and advisory compliance
2. Run `cargo audit` to scan for known CVE
3. Enable nightly fuzzing for VT parser and config parser
4. Add security.md contact once maintainer email is public
5. Shell hook scripts should not log/send command data to remote services
6. Data sent to local Ollama should be redacted by default
7. Default configuration should be local-only (no network except Ollama if explicitly enabled)
