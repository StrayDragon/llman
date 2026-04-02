# Promptfoo suites for llman

This folder contains Promptfoo fixtures + runner scripts used to evaluate llman SDD prompts and agentic workflows.

Entry points:
- `agentdev/promptfoo/run-sdd-prompts-eval.sh`: baseline vs candidate prompt A/B evaluation (chat-style)

Notes:
- Runner scripts create an isolated temp work directory under `./.tmp/` and use an isolated `LLMAN_CONFIG_DIR` to avoid touching real user config.
- `OPENAI_API_KEY` is required for running `promptfoo eval` (unless using `--no-gen`).
