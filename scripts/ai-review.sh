#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not a git repository"
    exit 1
fi

if git diff --stat --cached --quiet 2>/dev/null && git diff --stat --quiet 2>/dev/null; then
    echo "No changes to review (working tree is clean)."
    exit 0
fi

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  OpenCode AI Review${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

echo -e "${YELLOW}--- Unstaged changes ---${NC}"
git diff --stat
echo ""

echo -e "${YELLOW}--- Staged changes ---${NC}"
git diff --cached --stat 2>/dev/null || true
echo ""

# Use OpenCode to review the diff
PROMPT="Review the following git diff for SnarkTerm, a Rust GPU terminal emulator project.

Focus on:
- CRITICAL issues (bugs, security vulnerabilities, unsound code)
- Code quality issues (logic errors, incorrect error handling, panics)
- Security concerns (PTY handling, shell injection, data leaks, GPU/unsafe)
- Conventions (Rust idioms, project patterns, error handling style)

Do NOT comment on formatting, naming preferences, or style nits.
Do NOT suggest changes - just report what you find.

Here is the diff:"

DIFF_CONTENT=""
if ! git diff --cached --quiet 2>/dev/null; then
    DIFF_CONTENT=$(git diff --cached)
fi
if ! git diff --quiet 2>/dev/null; then
    DIFF_CONTENT="$DIFF_CONTENT"$'\n'"$(git diff)"
fi

exec opencode --input "$PROMPT"$'\n\n'"$DIFF_CONTENT" 2>&1
