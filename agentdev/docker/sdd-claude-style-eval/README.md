# sdd-claude-style-eval (Docker)

This docker runner builds a reproducible environment for the Claude Code agentic eval fixture:

- Entry: `scripts/sdd-claude-style-eval.sh`
- Output: mounted to `/repo/.tmp` inside the container

## Quick start

```bash
export ANTHROPIC_API_KEY=...

bash agentdev/docker/sdd-claude-style-eval/run.sh -- \
  --model sonnet \
  --max-turns 18 \
  --runs 1
```

## Aliyun mirrors (build args)

Example:

```bash
bash agentdev/docker/sdd-claude-style-eval/run.sh \
  --apt-mirror-debian http://mirrors.aliyun.com/debian \
  --apt-mirror-security http://mirrors.aliyun.com/debian-security \
  --npm-registry https://registry.npmmirror.com \
  --pip-index-url https://mirrors.aliyun.com/pypi/simple \
  -- --model sonnet --max-turns 18
```

## Notes

- This image enables an agentic workflow that can execute commands and write files inside its temp workspaces. Only run it against trusted inputs.
- Optional judge:
  - `--judge codex` requires `OPENAI_API_KEY`
