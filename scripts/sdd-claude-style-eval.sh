#!/usr/bin/env bash
set -euo pipefail

# Wrapper for:
# - `agentdev/promptfoo/run-sdd-claude-style-eval.sh`
# Notes:
# - Supports `--fixture v1|v2` (default v1)
# - When `--runs N (N>=2)`, runner writes batch aggregates under `./.tmp/`

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

exec bash "$REPO_ROOT/agentdev/promptfoo/run-sdd-claude-style-eval.sh" "$@"
