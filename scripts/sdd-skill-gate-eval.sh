#!/usr/bin/env bash
set -euo pipefail

# Wrapper for `agentdev/promptfoo/run-sdd-skill-gate-eval.sh`.
# Evaluates SDD skill template quality by rendering the SKILL.md into an
# agentic prompt + driving a sandbox task with a hard `llman sdd validate` gate.
# Supports baseline-vs-candidate A/B via `--baseline-skill <path>`.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

exec bash "$REPO_ROOT/agentdev/promptfoo/run-sdd-skill-gate-eval.sh" "$@"
