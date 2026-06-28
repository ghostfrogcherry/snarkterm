# SnarkTerm — OpenCode AI Development Loop

## How to Use This Repo with OpenCode

OpenCode is an AI coding assistant that operates in your terminal. This file describes the safe, effective workflow for using OpenCode on SnarkTerm.

### Setup

```bash
# Make AI scripts executable
chmod +x scripts/ai-*.sh
```

### Standard Workflow

```text
1. Prompt → 2. Read context → 3. Smallest change → 4. Validate → 5. Show status
```

### Scripts

| Script | Purpose |
|--------|---------|
| `scripts/ai-test.sh` | Validate project (cargo check / cargo fmt + check) |
| `scripts/ai-task.sh` | Execute an AI task with full loop |
| `scripts/ai-review.sh` | Review current diff without editing |

### Example Prompts

```text
# Quick validation
Run scripts/ai-test.sh

# Feature task
scripts/ai-task.sh "Add unit tests for the VT parser in snarkterm-core"

# Code review
scripts/ai-review.sh

# Specific task
Add error handling to the PTY read loop in crates/snarkterm-pty/src/pty.rs

# Review work
Run scripts/ai-test.sh and show me the diff
```

### Safe Workflow Rules

1. **Read first** — Always read AGENTS.md, relevant source files, and docs before editing
2. **Smallest change** — Make the minimal change needed; don't refactor unrelated code
3. **Validate** — Always run `scripts/ai-test.sh` after making changes
4. **Never commit unless asked** — Show status with `git diff` and `git status`
5. **Never install dependencies** — Don't run `cargo install` or `apt install` globally
6. **Never delete/rename files** — Only create or edit files
7. **Check conventions** — Follow existing code style, naming, and patterns

### Context Files to Read First

| File | Why |
|------|-----|
| `AGENTS.md` | Project overview, architecture, paths |
| `PROJECT_STATUS.md` | Current state, what's missing |
| `docs/ARCHITECTURE.md` | Full architecture, dataflow, design |
| `docs/ROADMAP.md` | Phased development plan |
| `SECURITY.md` | Security-sensitive areas |
| `Cargo.toml` | Workspace structure, dependencies |

### LLM Integration Note

When using OpenCode as your AI loop driver, the workflow is:

1. OpenCode reads `AGENTS.md` first to understand project conventions
2. Task is described in the prompt
3. OpenCode reads relevant source
4. Makes smallest change
5. Runs `cargo check --workspace` to validate
6. Reports what was done

This ensures consistent, safe, and reviewable AI-assisted development.
