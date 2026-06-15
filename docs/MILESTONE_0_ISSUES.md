# First Milestone Issue List

Milestone: `0.1.0-alpha.0 - It Probably Works On Your Machine`

1. Create CI workflow for `cargo fmt`, `cargo clippy`, and `cargo test`.
2. Add workspace crate skeletons and dependency policy.
3. Implement config loading with defaults and validation.
4. Create app window with `winit`.
5. Initialize `wgpu` surface, adapter, device, and queue.
6. Spawn a shell with `portable-pty`.
7. Implement PTY read/write channels.
8. Feed PTY bytes into a minimal VT parser.
9. Render a fixed-size terminal grid.
10. Encode keyboard input and send it to the PTY.
11. Implement resize handling from window to PTY.
12. Add OSC 777 parser for SnarkTerm shell markers.
13. Add Bash shell integration script.
14. Add Zsh shell integration script.
15. Add Fish shell integration script.
16. Define `TerminalEvent` and commentary channel.
17. Implement first deterministic rules: sudo, force push, chmod 777, curl pipe shell, dangerous rm.
18. Render initial snark gutter as separate UI layer.
19. Add SQLite migration runner.
20. Add man page and TLDR page packaging targets.
