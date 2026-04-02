# Promptfoo suites for llman

This folder contains Promptfoo fixtures + runner scripts used to evaluate llman SDD prompts and agentic workflows.

Entry points:
- `agentdev/promptfoo/run-sdd-prompts-eval.sh`: baseline vs candidate prompt A/B evaluation (chat-style)
- `agentdev/promptfoo/run-sdd-claude-style-eval.sh`: Claude Code agentic multi-style eval (ison/toon/yaml, with hard gate)

Notes:
- Runner scripts create an isolated temp work directory under `./.tmp/` and use an isolated `LLMAN_CONFIG_DIR` to avoid touching real user config.
- `OPENAI_API_KEY` is required when using OpenAI models for `promptfoo eval` or judge grading.
- `ANTHROPIC_API_KEY` (or equivalent Claude Code env) is required for the Claude Code agentic eval.
- For `anthropic:claude-agent-sdk` provider, you must install local deps once:
  - `pnpm -C agentdev/promptfoo install`
  - (node_modules is ignored; do not commit secrets)

Examples:
- Local dry-run (no API calls): `bash scripts/sdd-claude-style-eval.sh --no-run`
- With Claude Code account env injection (sensitive): `bash scripts/sdd-claude-style-eval.sh --cc-account glm-lite-150`
