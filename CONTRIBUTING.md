# Contributing

SnarkTerm is currently in the architecture and planning stage.

Expected checks once Rust implementation begins:

```text
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo deny
cargo audit
```

Terminal compatibility work should include smoke testing with `vttest`, `tmux`, `vim`, `less`, `htop`, `fzf`, and common shells.
