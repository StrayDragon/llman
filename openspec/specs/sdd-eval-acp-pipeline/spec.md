# sdd-eval-acp-pipeline Specification

## Purpose
Provide an experimental `llman x sdd-eval` pipeline that can run scripted, repeatable SDD evaluations across multiple workflow styles (`sdd` vs `sdd-legacy`) and multiple ACP agents (Claude Code / Codex), producing comparable artifacts and reports without leaking secrets.
## Requirements
### Requirement: `llman x sdd-eval` command is available
The CLI MUST provide the experimental subcommand `llman x sdd-eval` with playbook-driven execution.

#### Scenario: Help is available
- **WHEN** the user runs `llman x sdd-eval --help`
- **THEN** the CLI prints help text and exits successfully

### Requirement: Playbooks are managed under the project `.llman/` directory
By default, `llman x sdd-eval init` MUST create playbooks under `<project>/.llman/sdd-eval/playbooks/`.
`llman x sdd-eval run` MUST accept an explicit playbook path, and MUST resolve relative paths from the current working directory.

#### Scenario: Init writes a YAML template playbook
- **WHEN** the user runs `llman x sdd-eval init --name demo` in a project root
- **THEN** `<project>/.llman/sdd-eval/playbooks/demo.yaml` exists and is parseable YAML
- **AND** the first non-empty line of the template is `# yaml-language-server: $schema=https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/playbooks/en/llman-sdd-eval.schema.json`
- **AND** the template uses the workflow/jobs/steps playbook structure (including `workflow.jobs`)

### Requirement: Runs are isolated and stored under `.llman/sdd-eval/runs/<run_id>/`
Each `llman x sdd-eval run` invocation MUST create a new run directory under `<project>/.llman/sdd-eval/runs/<run_id>/`.
The run directory MUST contain:
- `<run_dir>/manifest.json` (machine-readable run manifest including playbook metadata and variant list)
- `<run_dir>/playbook.yaml` (a copy of the playbook used for the run)
- `<run_dir>/variants/<variant_id>/workspace/` for each defined variant
- `<run_dir>/variants/<variant_id>/logs/` for each defined variant
- `<run_dir>/variants/<variant_id>/artifacts/` for each defined variant

The runner MUST create the run directory, write the manifest/playbook copy, and create the per-variant directory layout BEFORE executing any workflow steps.

#### Scenario: Run creates a new run directory and base layout
- **WHEN** the user runs `llman x sdd-eval run --playbook <path>`
- **THEN** a new `<run_id>` directory exists under `<project>/.llman/sdd-eval/runs/`
- **AND** `manifest.json` exists under the run directory
- **AND** `playbook.yaml` exists under the run directory
- **AND** each defined variant has `workspace/`, `logs/`, and `artifacts/` directories under `<run_dir>/variants/`

### Requirement: Variants combine workflow style and agent preset
A playbook MUST define one or more `variants` that the workflow can execute.

Each variant MUST specify:
- a workflow `style` (`sdd` or `sdd-legacy`)
- an ACP `agent` definition (e.g. `claude-code-acp` or `codex-acp`)
- an account preset reference (Claude Code: `llman x cc` group; Codex: `llman x codex` group)

Variants MUST be addressable by a stable id (for example `variants.a`, `variants.b`) so that jobs can reference them via `strategy.matrix.variant`.

#### Scenario: Missing variants fails loudly
- **WHEN** the playbook has no variants and the user runs `llman x sdd-eval run ...`
- **THEN** the command exits non-zero and explains that at least one variant is required

#### Scenario: Matrix references unknown variant fails loudly
- **WHEN** a job defines `strategy.matrix.variant: ["a"]`
- **AND** the playbook does not define `variants.a`
- **THEN** the command exits non-zero
- **AND** the error explains that the matrix references a missing variant id

### Requirement: Workflow initialization is performed per variant
For each variant workspace, the runner MUST be able to initialize SDD templates corresponding to the variant workflow style:
- for `sdd`: initialize in “new” style
- for `sdd-legacy`: initialize in “legacy” style

In workflow-based playbooks, initialization MUST be exposed as a built-in action (for example `builtin:sdd-eval/sdd.prepare`) so it can be composed as an explicit step.

#### Scenario: Legacy variant produces legacy templates
- **WHEN** a variant uses style `sdd-legacy`
- **THEN** the variant workspace is initialized using legacy SDD templates (equivalent to `llman sdd-legacy init` + `llman sdd-legacy update`)

### Requirement: ACP agents are launched with preset env injection (without leaking secrets)
The runner MUST support launching ACP agent processes for Claude Code and Codex.
For each variant, the runner MUST resolve the referenced preset and inject its environment variables into the spawned ACP agent process.
The runner MUST NOT print or write secret values to:
- stdout/stderr
- playbook files or “resolved playbook” copies
- run manifests or per-variant artifacts/logs

Only the preset identifier (group name) and environment variable **names** MAY be recorded for debugging.

#### Scenario: Run artifacts never include an API key value
- **WHEN** the user runs `llman x sdd-eval run ...` with a preset that includes an API key env var
- **THEN** no file under `<project>/.llman/sdd-eval/runs/<run_id>/` contains the raw API key value

### Requirement: The ACP runner is sandboxed to the variant workspace
The ACP client implementation MUST restrict file operations and terminal commands to the variant workspace.
It MUST deny attempts to read/write outside the workspace root.

#### Scenario: Path traversal is rejected
- **WHEN** the agent requests reading `../../.ssh/id_rsa`
- **THEN** the client denies the request and records a non-secret error in the variant log

### Requirement: Evaluation runs are iteration-bounded and reproducible
The playbook MUST define a fixed maximum iteration count for the SDD loop execution via `sdd_loop.max_iterations`.
If `sdd_loop.max_iterations` is omitted, the runner MUST default to 6.
If `sdd_loop.max_iterations` is present, it MUST be an integer > 0.

The runner MUST stop the SDD loop after the configured number of iterations (no “auto completion” detection in v1).

#### Scenario: Loop stops at max iterations
- **WHEN** max iterations is set to 3
- **THEN** the runner performs at most 3 iterations and then marks the variant as completed-by-limit

### Requirement: Reports include comparable objective metrics
`llman x sdd-eval report --run <run_id>` MUST generate a report that includes, per variant:
- number of iterations executed
- file change summary (counts and sizes)
- terminal command summary (commands, exit codes, duration, truncated output)

#### Scenario: Report is generated after a run
- **WHEN** the user runs `llman x sdd-eval report --run <run_id>`
- **THEN** the command writes a report file under the run directory

### Requirement: Human scoring export and import are supported
The reporting workflow MUST support exporting a “human scoring pack” (JSON + pointers to relevant artifacts).
It MUST support importing a human scoring result file and merging it into the report outputs.

#### Scenario: Human scores can be imported
- **WHEN** the user runs `llman x sdd-eval import-human --run <run_id> --file scores.json`
- **THEN** the report data for `<run_id>` is updated to include the imported scores

### Requirement: AI Judge scoring via OpenAI-compatible API is optional
If enabled in the playbook, the report step MUST be able to call an OpenAI-compatible API using `OPENAI_*` environment variables to produce AI-judge scores.
If disabled, the runner MUST NOT require `OPENAI_*` variables.

#### Scenario: Missing OPENAI key fails only when AI judge is enabled
- **WHEN** `OPENAI_API_KEY` is empty and AI judge is enabled
- **THEN** `llman x sdd-eval report ...` exits non-zero and explains that `OPENAI_API_KEY` is required for AI judge
