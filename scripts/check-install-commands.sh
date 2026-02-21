#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

find_fixed() {
  local pattern="$1"
  shift
  if command -v rg >/dev/null 2>&1; then
    rg -nF "$pattern" "$@"
  else
    grep -R -n -F -- "$pattern" "$@"
  fi
}

find_regex() {
  local pattern="$1"
  shift
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$@"
  else
    grep -R -n -E -- "$pattern" "$@"
  fi
}

assert_contains() {
  local pattern="$1"
  shift
  if ! find_fixed "$pattern" "$@" >/dev/null; then
    fail "Missing required install command: ${pattern}"
  fi
}

# Front-facing command requirements
assert_contains "curl -fsSL https://agentralabs.tech/install/vision | bash" README.md docs/quickstart.md
assert_contains "cargo install agentic-vision-mcp" README.md docs/quickstart.md
assert_contains "cargo add agentic-vision" README.md docs/quickstart.md

# Invalid patterns
if find_regex "cargo install agentic-vision agentic-vision-mcp" README.md docs >/dev/null; then
  fail "Found invalid combined cargo install for vision library + MCP"
fi
if find_regex "^cargo install agentic-vision$" README.md docs >/dev/null; then
  fail "Found invalid binary install command for library crate agentic-vision"
fi
if find_regex "cargo install --git .*agentic-vision( |$)" scripts/install.sh >/dev/null; then
  fail "Vision installer must not cargo-install the library crate (agentic-vision)"
fi

# Installer health
bash -n scripts/install.sh
bash scripts/install.sh --dry-run >/dev/null

# Public endpoint/package health
curl -fsSL https://agentralabs.tech/install/vision >/dev/null
curl -fsSL https://crates.io/api/v1/crates/agentic-vision >/dev/null
curl -fsSL https://crates.io/api/v1/crates/agentic-vision-mcp >/dev/null

echo "Install command guardrails passed (vision)."
