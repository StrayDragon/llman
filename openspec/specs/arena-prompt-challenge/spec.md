# arena-prompt-challenge Specification

## Purpose
Provide an experimental `llman x arena` workflow to evaluate prompt variants (and optionally multiple models) on a shared task set, using batch generation + human voting + Elo ratings. Support both direct output evaluation (`text`) and repo-based objective signals (`repo`: patch apply + verification commands) in an isolated temp workspace.

## Requirements

### Requirement: `llman x arena` command is available
The CLI MUST provide the experimental subcommand `llman x arena`.

#### Scenario: Help is available
- **WHEN** the user runs `llman x arena --help`
- **THEN** the CLI prints help text for the arena command and exits successfully

### Requirement: Arena data is stored under `LLMAN_CONFIG_DIR`
All arena artifacts (contests, datasets, runs) MUST be stored under `<LLMAN_CONFIG_DIR>/arena/`.

#### Scenario: Config dir override isolates arena data
- **WHEN** `LLMAN_CONFIG_DIR` is set to `/tmp/llman-test-config` and the user creates a contest/dataset/run
- **THEN** all created files are under `/tmp/llman-test-config/arena/`

### Requirement: Model discovery uses `OPENAI_*` environment variables
`llman x arena models list` MUST read `OPENAI_API_KEY` and MAY read `OPENAI_BASE_URL`.
It MUST request the OpenAI-compatible endpoint `GET <base>/v1/models` and MUST print model ids to stdout.

#### Scenario: Missing API key fails loudly
- **WHEN** `OPENAI_API_KEY` is unset or empty and the user runs `llman x arena models list`
- **THEN** the command exits non-zero and explains that `OPENAI_API_KEY` is required

### Requirement: Interactive model multi-select is available
`llman x arena models pick` MUST provide an interactive multi-select UI and MUST print the selected model ids as a JSON array to stdout (copy/paste friendly).

#### Scenario: Selected models are printed as JSON
- **WHEN** the user selects two models in `llman x arena models pick`
- **THEN** stdout is a JSON array like `["model-a","model-b"]`

### Requirement: Contest initialization writes a runnable template
`llman x arena contest init --name <NAME>` MUST create a contest template file at `<LLMAN_CONFIG_DIR>/arena/contests/<NAME>.toml`.
The template MUST be parseable and runnable after the user fills required fields.

#### Scenario: Contest template exists
- **WHEN** the user runs `llman x arena contest init --name demo`
- **THEN** `<LLMAN_CONFIG_DIR>/arena/contests/demo.toml` exists

### Requirement: Dataset initialization writes a runnable template
`llman x arena dataset init --name <NAME>` MUST create a dataset template file at `<LLMAN_CONFIG_DIR>/arena/datasets/<NAME>.yaml`.
The template MUST include at least one `text` task and one `repo` task example.

#### Scenario: Dataset template exists
- **WHEN** the user runs `llman x arena dataset init --name demo`
- **THEN** `<LLMAN_CONFIG_DIR>/arena/datasets/demo.yaml` exists

### Requirement: Prompts are loaded from llman prompt storage
Arena contestants MUST reference prompts by name and app, and the implementation MUST load prompt content using the same prompt storage as `llman prompts` (i.e. under `<LLMAN_CONFIG_DIR>/prompt/<app>/`).

#### Scenario: Missing prompt fails
- **WHEN** a contest references a `prompt_name` that does not exist in `<LLMAN_CONFIG_DIR>/prompt/<app>/`
- **THEN** `llman x arena gen ...` exits non-zero and reports the missing prompt name

### Requirement: Generation creates a run directory with JSONL records
`llman x arena gen --contest <CONTEST> --dataset <DATASET> --rounds <N>` MUST create a run directory under `<LLMAN_CONFIG_DIR>/arena/runs/<RUN_ID>/` and MUST write:
- `matches.jsonl`
- `generations.jsonl`

#### Scenario: Run artifacts are created
- **WHEN** the user runs `llman x arena gen --contest c1 --dataset d1 --rounds 3`
- **THEN** a new `<RUN_ID>` directory exists with `matches.jsonl` and `generations.jsonl`

### Requirement: Repo tasks require a repo template directory
If a dataset contains any `repo` tasks, the dataset MUST define `repo_template_path`.

#### Scenario: Missing repo template path fails
- **WHEN** the dataset contains a `repo` task but does not define `repo_template_path`
- **THEN** `llman x arena gen ...` exits non-zero and reports that `repo_template_path` is required for `repo` tasks

### Requirement: Repo task execution is isolated in a temp workspace
For `repo` tasks, arena MUST copy `repo_template_path` into a temp directory workspace and MUST run all patch apply and verification commands with `cwd` set to that workspace.
Arena MUST NOT modify the repo template directory in place.

#### Scenario: Repo template is not modified
- **WHEN** the user runs `llman x arena gen ...` for a dataset with `repo` tasks
- **THEN** changes are applied only in the temp workspace and not written back to `repo_template_path`

### Requirement: Repo tasks are evaluated by patch apply and verification commands
For each `repo` task generation, the model output MUST be treated as a unified diff.
Arena MUST attempt to apply it (e.g. via `git apply`) and MUST record an apply result.
If apply succeeds, arena MUST run verification commands and MUST record exit codes and output summaries.
If apply fails, arena MUST skip verification and MUST record that verification was skipped.

#### Scenario: Apply failure skips verification
- **WHEN** a repo task generation produces an invalid diff that fails to apply
- **THEN** the run records an apply failure and the verification result is recorded as skipped

### Requirement: Human voting is recorded and resumable
`llman x arena vote --run <RUN_ID>` MUST replay matches interactively and MUST append vote records to `votes.jsonl`.
If `votes.jsonl` already contains votes for some matches, the command MUST skip those matches and continue with the remaining ones.

#### Scenario: Vote resumes after interruption
- **WHEN** the user votes on the first 2 matches, exits, then runs `llman x arena vote --run <RUN_ID>` again
- **THEN** the command continues from the next unvoted match without duplicating votes

### Requirement: Elo report is generated from votes
`llman x arena report --run <RUN_ID>` MUST compute Elo ratings from recorded votes and MUST write:
- `ratings.json`
- `report.md`
The report MUST include a leaderboard sorted by rating.

#### Scenario: Report requires at least one vote
- **WHEN** the run has no votes and the user runs `llman x arena report --run <RUN_ID>`
- **THEN** the command exits non-zero and reports that at least one vote is required to compute ratings

### Requirement: Network/LLM subcommands are not feature-gated
Arena subcommands requiring network/LLM support (including `models list`, `models pick`, and `gen`) MUST be available in the default build and MUST be listed in `--help` output.

#### Scenario: Help lists network/LLM subcommands
- **WHEN** the user runs `llman x arena --help`
- **THEN** help output lists the `models` and `gen` subcommands
