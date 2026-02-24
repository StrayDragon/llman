## 1. CLI & data layout scaffolding

- [x] 1.1 Add `llman x arena` clap wiring and module skeleton (`src/x/arena/**`)
- [x] 1.2 Define arena data root at `<LLMAN_CONFIG_DIR>/arena/` and helper path functions
- [x] 1.3 Add serde structs + loaders for contest TOML and dataset YAML (with validation errors)

## 2. Init commands (contest/dataset)

- [x] 2.1 Implement `llman x arena contest init --name <NAME> [--force]` writing `<config>/arena/contests/<NAME>.toml`
- [x] 2.2 Implement `llman x arena dataset init --name <NAME> [--force]` writing `<config>/arena/datasets/<NAME>.yaml` (include text+repo examples)
- [x] 2.3 Non-interactive overwrite policy: refuse without `--force` and provide actionable error messages

## 3. Models discovery (`/v1/models`)

- [x] 3.1 Implement `llman x arena models list [--json]` using `OPENAI_API_KEY` + optional `OPENAI_BASE_URL`
- [x] 3.2 Normalize base url and request `GET <base>/v1/models`; parse response and print model ids
- [x] 3.3 Implement `llman x arena models pick` (MultiSelect) and print selected ids as JSON array

## 4. Generation core (`gen`)

- [x] 4.1 Add required dependencies for arena network/LLM support in the default build (no feature gating) and wire them into the arena runner
- [x] 4.2 Implement prompt loading from `<LLMAN_CONFIG_DIR>/prompt/<app>/` consistent with `llman prompts`
- [x] 4.3 Expand contestants as `prompt × model` and define stable contestant ids (e.g. `p1@model`)
- [x] 4.4 Implement match-making for `--rounds N` with `--seed` determinism; write `matches.jsonl`
- [x] 4.5 Implement LLM runner (OpenAI-compatible) to generate `text` outputs and `repo` patch diffs; write `generations.jsonl`

## 5. Repo task objective evaluation

- [x] 5.1 Validate `repo_template_path` is present when dataset contains `repo` tasks
- [x] 5.2 Copy `repo_template_path` into a temp workspace; ensure repo template is never modified in place
- [x] 5.3 Apply diff via `git apply`; record apply result (`applies.jsonl`)
- [x] 5.4 Run verification commands (contest default, task override) only when apply succeeds; record exit codes and output summaries (`verifications.jsonl`)

## 6. Voting & reporting

- [x] 6.1 Implement `llman x arena vote --run <RUN_ID>`: replay matches, display A/B outputs (+ repo apply/verify results), append to `votes.jsonl`
- [x] 6.2 Make voting resumable: skip matches already present in `votes.jsonl`
- [x] 6.3 Implement Elo computation and `llman x arena report --run <RUN_ID>` writing `ratings.json` and `report.md`

## 7. Tests & docs

- [x] 7.1 Unit tests: base url normalization, contestant expansion, Elo updates
- [x] 7.2 Integration tests with a fake runner: `gen` creates run artifacts; repo apply/verify behavior; vote resume; report errors when no votes
- [x] 7.3 Add minimal user docs/help examples for `llman x arena` (README or `docs/`), including required env vars and sample workflow
