#!/bin/bash
# Launch a persistent Claude Code session for Cortex
# Keeps the Mac awake and survives display sleep
#
# Usage:
#   ./scripts/claude-session.sh              # attach or create session
#   ./scripts/claude-session.sh -k           # kill the session

SESSION="cortex"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if [[ "$1" == "-k" ]]; then
  tmux kill-session -t "$SESSION" 2>/dev/null && echo "Session '$SESSION' killed."
  exit 0
fi

if tmux has-session -t "$SESSION" 2>/dev/null; then
  echo "Attaching to existing session '$SESSION'..."
  tmux attach-session -t "$SESSION"
else
  echo "Starting new session '$SESSION' with caffeinate..."
  # caffeinate -i prevents system idle sleep for the duration of the child process
  tmux new-session -d -s "$SESSION" -c "$PROJECT_DIR" \
    "caffeinate -i bash --login"
  tmux send-keys -t "$SESSION" "claude" Enter
  tmux attach-session -t "$SESSION"
fi
