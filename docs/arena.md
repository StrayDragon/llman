# Arena (experimental)

`llman x arena` is an experimental workflow for running prompt/model “battles” on a shared dataset:

- Batch generation (`gen`)
- Human voting (`vote`)
- Elo ratings + Markdown report (`report`)

It supports two task kinds:

- `text`: vote directly on outputs
- `repo`: model outputs a unified diff → arena applies it in a temp workspace → runs verification commands → shows objective signals during voting

## Environment variables

Model discovery and OpenAI-compatible generation use `OPENAI_*`:

- `OPENAI_API_KEY` (required for `models` and `gen`)
- `OPENAI_BASE_URL` (optional, defaults to `https://api.openai.com`)
  - `OPENAI_API_BASE` is also accepted
  - `/v1` is appended automatically if missing

## Data layout

All arena data is stored under `<LLMAN_CONFIG_DIR>/arena/`:

- `contests/` — contest TOML configs
- `datasets/` — dataset YAML configs
- `runs/` — run artifacts (`matches.jsonl`, `generations.jsonl`, `votes.jsonl`, …)

Prompts are loaded from the same storage as `llman prompts`:

- `<LLMAN_CONFIG_DIR>/prompt/codex/*.md`
- `<LLMAN_CONFIG_DIR>/prompt/claude-code/*.txt`

Use `--config-dir` (or `LLMAN_CONFIG_DIR`) to isolate all files when testing.

## Quick start

1) Create prompts (example for `codex`)

```bash
llman prompts upsert --app codex --name p1 --content "You are a helpful coding assistant."
llman prompts upsert --app codex --name p2 --content "Be strict. Output only what is asked."
```

2) Pick models

```bash
llman x arena models list
llman x arena models pick
```

3) Create a contest + dataset template, then edit them

```bash
llman x arena contest init --name demo
llman x arena dataset init --name demo
```

Notes:
- `contest.verify` defines default verification commands for `repo` tasks.
- If any dataset task is `type: repo`, you must set `repo_template_path`.
- `contest` also lets you pin request params (`temperature`, `top_p`, `top_k`, `max_output_tokens`, `timeout_secs`, `retries`).
- If `contest.structured_output = true`, arena requests JSON schema `{ output: string }` and extracts `output` as the effective generation. Use `repair_retries` to retry invalid structured output.

4) Generate a run

```bash
llman x arena gen --contest demo --dataset demo --rounds 10 --seed 42
```

5) Vote and report

```bash
llman x arena vote --run run_42
llman x arena report --run run_42
```

## Repo tasks (diff-only contract)

For `repo` tasks, the model must output **only** a unified diff (`git apply` compatible), with no commentary.

Verification commands are executed in the temp workspace created from `repo_template_path` (the template is never modified in place).
