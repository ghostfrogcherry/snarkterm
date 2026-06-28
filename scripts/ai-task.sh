#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <task description>"
    echo ""
    echo "Runs OpenCode with a task prompt that:"
    echo "  1. Reads AGENTS.md for project context"
    echo "  2. Makes the smallest possible change"
    echo "  3. Runs ai-test.sh after changes"
    echo "  4. Shows git diff and status"
    exit 1
fi

TASK="$*"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Colors
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  OpenCode AI Task${NC}"
echo -e "${CYAN}========================================${NC}"
echo "Project: $PROJECT_ROOT"
echo "Task:    $TASK"
echo ""

# Build the prompt that follows the safe workflow
PROMPT="I am working on SnarkTerm ($PROJECT_ROOT). Before you make changes:

1. Read AGENTS.md and PROJECT_STATUS.md first
2. Read the relevant source files before editing
3. Make the SMALLEST possible change to accomplish this task
4. After making changes, run './scripts/ai-test.sh'
5. Show me the git diff and git status when done
6. Do NOT commit, do NOT install dependencies, do NOT delete files

Task: $TASK"

exec opencode --input "$PROMPT" 2>&1
