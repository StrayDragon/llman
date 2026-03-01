## ADDED Requirements

### Requirement: Playbook Uses a Workflow/Jobs/Steps Model
The `llman x sdd-eval` playbook MUST be a YAML document that defines a workflow to execute.

The playbook MUST be a YAML mapping.

The playbook MUST define:
- `task` (the shared evaluation task)
- `variants` (the AB groups to evaluate)
- `workflow.jobs` (the jobs and steps to run)

`workflow` MUST be a YAML mapping and MUST define:
- `jobs` (a mapping of job ids to job definitions)

Each `job_id` under `workflow.jobs` MUST match `^[a-zA-Z][a-zA-Z0-9_-]*$`.

Each job definition MUST be a YAML mapping and MUST define:
- `steps` (a non-empty array)

Each job definition MAY define:
- `needs` (an array of `job_id`)
- `strategy.matrix.variant` (an array of `variant_id`)

Unknown keys under `workflow`, `workflow.jobs.<job_id>`, `strategy`, and `matrix` MUST be rejected.

The playbook MAY define:
- `name` (a human-readable name)
- `report` (report configuration)
- `sdd_loop` (default SDD loop configuration)

Unknown top-level keys MUST be rejected.

#### Scenario: Missing workflow fails loudly
- **WHEN** a user runs `llman x sdd-eval run --playbook <path>` with a playbook missing `workflow.jobs`
- **THEN** the command exits non-zero
- **AND** the error explains that `workflow.jobs` is required

#### Scenario: Empty job steps are rejected
- **WHEN** a playbook job defines `steps: []`
- **THEN** playbook validation fails
- **AND** the error explains that `steps` must be non-empty

#### Scenario: Unknown job keys are rejected
- **WHEN** a playbook job defines an unknown key (for example `timeout:`)
- **THEN** playbook validation fails
- **AND** the error indicates an unknown field

### Requirement: Variants Are Addressable by Stable Id and Expandable by Matrix
The playbook MUST define `variants` as a mapping from a stable `variant_id` to a variant configuration.

Each `variant_id` MUST:
- match `^[a-zA-Z][a-zA-Z0-9_-]*$`
- be used as the directory name under `<run_dir>/variants/<variant_id>/`

Each variant configuration MUST define:
- `style`: `sdd` or `sdd-legacy`
- `agent`: an ACP agent definition (kind/preset + optional command/args)

A job MAY define `strategy.matrix.variant` as an array of `variant_id`.
If present, the runner MUST expand the job into one job-execution per listed `variant_id`.
Expanded executions MUST run sequentially in the order listed in `strategy.matrix.variant` (no parallelism in this change).

#### Scenario: Matrix references an unknown variant
- **WHEN** a playbook job defines `strategy.matrix.variant: ["a"]`
- **AND** the playbook does not define `variants.a`
- **THEN** the command exits non-zero
- **AND** the error explains that the matrix references a missing variant id

#### Scenario: Variant id is rejected when unsafe for paths
- **WHEN** a user defines a variant id containing `/` or `..`
- **THEN** playbook validation fails
- **AND** the error explains the allowed `variant_id` pattern

#### Scenario: Matrix variant expansion is serial and ordered
- **WHEN** a job defines `strategy.matrix.variant: ["b", "a"]`
- **THEN** the runner executes the job twice
- **AND** it runs the `b` execution before the `a` execution

### Requirement: Job `needs` Dependencies Are Resolved Deterministically
Jobs MAY declare `needs: [job_id...]` to depend on other jobs.

The runner MUST:
- reject a `needs` entry that references an unknown job id
- reject dependency cycles
- execute jobs in a deterministic topological order
- when multiple jobs are runnable, use YAML declaration order as a stable tie-breaker

#### Scenario: `needs` references an unknown job
- **WHEN** a job declares `needs: ["missing"]`
- **THEN** the playbook fails validation
- **AND** the error mentions the missing job id

#### Scenario: `needs` cycle is rejected
- **WHEN** the workflow contains a dependency cycle (direct or indirect)
- **THEN** the playbook fails validation
- **AND** the error explains that cycles are not allowed

### Requirement: Step Kind Is Either `uses` or `run`
Each job defines an ordered list of `steps`.
Each step MUST specify exactly one of:
- `uses`: reference a built-in action
- `run`: execute a local command

Each step MUST be a YAML mapping.

Allowed step keys are:
- `name` (optional string)
- `uses` (string) and `with` (optional mapping)
- `run` (string) and `cwd` (optional string)

If `uses` is present:
- `with` MAY be present
- `cwd` MUST NOT be present

If `run` is present:
- `cwd` MAY be present
- `with` MUST NOT be present

Unknown step keys MUST be rejected.

Steps MUST be executed sequentially in the order declared.

#### Scenario: Step with both `uses` and `run` is rejected
- **WHEN** a step defines both `uses` and `run`
- **THEN** playbook validation fails
- **AND** the error explains that exactly one step kind is allowed

#### Scenario: `run` step with `with` is rejected
- **WHEN** a step defines `run: "rg foo"` and also defines `with: {}`
- **THEN** playbook validation fails
- **AND** the error explains that `with` is only allowed for `uses` steps

### Requirement: Built-In Actions Are Supported
The runner MUST support the following built-in actions (stable identifiers):
- `builtin:sdd-eval/workspace.prepare`
- `builtin:sdd-eval/sdd.prepare`
- `builtin:sdd-eval/acp.sdd-loop`
- `builtin:sdd-eval/report.generate`

If a step references an unknown built-in action, the runner MUST fail loudly.

#### Scenario: Unknown built-in action fails loudly
- **WHEN** a step uses `builtin:sdd-eval/does-not-exist`
- **THEN** the command exits non-zero
- **AND** the error explains that the action is unknown

### Requirement: Built-In Actions Have Stable Semantics and Outputs
Each built-in action MUST operate only within its sandbox root:
- for matrix-expanded jobs: the variant workspace root (`<run_dir>/variants/<variant_id>/workspace/`)
- for non-matrix jobs: the run directory root (`<run_dir>/`)

Built-in actions MUST NOT read/write outside their sandbox root.

The following stable semantics MUST be implemented:

- `builtin:sdd-eval/workspace.prepare`
  - MUST copy the project root into the variant workspace.
  - MUST NOT copy:
    - `.git/`, `.llman/`, `target/`, `node_modules/`, `.venv/`, `dist/`, `build/`
    - secret-ish files: `.env`, `.env.*`, `.npmrc`, `.pypirc`, `.netrc`
  - MUST ensure `logs/` and `artifacts/` directories exist for the variant.

- `builtin:sdd-eval/sdd.prepare`
  - MUST initialize/update SDD templates inside the workspace according to `variant.style`:
    - `sdd` → new style
    - `sdd-legacy` → legacy style

- `builtin:sdd-eval/acp.sdd-loop`
  - MUST resolve the variant agent preset and inject env vars into the spawned ACP agent process only.
  - MUST NOT print or write secret values; only env var names MAY be recorded.
  - MUST write:
    - `<run_dir>/variants/<variant_id>/logs/acp-session.jsonl`
    - `<run_dir>/variants/<variant_id>/artifacts/acp-metrics.json`

- `builtin:sdd-eval/report.generate`
  - MUST generate (or re-generate) the run report under `<run_dir>/` using the same behavior as `llman x sdd-eval report --run <run_id>`.

Built-in actions MUST reject unknown `with` keys.
For this change, built-in actions MUST define no `with` keys: `with` MAY be omitted or an empty mapping.

#### Scenario: Workspace prepare skips secret-ish files
- **WHEN** the project root contains `.env` or `.netrc`
- **AND** the workflow runs `builtin:sdd-eval/workspace.prepare` for a variant
- **THEN** those files are not present in the variant workspace

#### Scenario: ACP loop action writes expected artifacts
- **WHEN** the workflow runs `builtin:sdd-eval/acp.sdd-loop` for a variant
- **THEN** `acp-session.jsonl` exists under the variant `logs/`
- **AND** `acp-metrics.json` exists under the variant `artifacts/`

### Requirement: Minimal Context Interpolation Is Supported
The runner MUST support simple string interpolation using the syntax `${{ <path> }}`.

Interpolation MUST be a pure string substitution (no expressions, operators, or functions).
Whitespace around `<path>` inside `${{ ... }}` MAY be present and MUST be ignored.

The runner MUST apply interpolation before executing a step, at minimum for:
- `run` step fields (`run`, and `cwd` if present)
- the `uses` value
- all string values inside `with` (recursively)

At minimum, the following paths MUST be supported:
- `matrix.variant` (current variant id for matrix-expanded jobs)
- `variant.style`
- `variant.agent.kind`
- `task.title`
- `task.prompt`
- `run.run_id`
- `run.run_dir`

If a path requires a context that is not present (for example `matrix.variant` in a non-matrix job), the runner MUST fail loudly.
Unknown interpolation paths MUST fail loudly (no silent passthrough).

#### Scenario: Interpolation replaces matrix.variant in run steps
- **WHEN** a matrix-expanded job step uses `run: "echo ${{ matrix.variant }}"`
- **THEN** the runner interpolates the string before execution
- **AND** the executed argv contains the concrete variant id value

#### Scenario: Unknown interpolation path fails loudly
- **WHEN** a step string contains `${{ does.not.exist }}`
- **THEN** the command exits non-zero
- **AND** the error explains that the interpolation path is unknown

### Requirement: `run` Steps Execute Allowlisted Commands Under Sandbox
The runner MUST support `run` steps for workflow jobs.

`run` steps MAY define an optional `cwd` field, interpreted as a relative path inside the sandbox root.
If omitted, `cwd` defaults to the sandbox root.

The runner MUST:
- interpret `run` as a single command invocation (no newline; no shell operators)
- parse `run` into argv using deterministic POSIX-style shellwords tokenization (single/double quotes and backslash escapes; unmatched quotes MUST fail)
- reject any parsed argv that contains shell operator tokens (`&&`, `||`, `|`, `;`, `>`, `<`)
- enforce a command allowlist based on the basename of `argv[0]` (same policy and command set as the ACP terminal allowlist)
- restrict execution cwd to a predefined sandbox root:
  - for matrix-expanded jobs: the variant workspace root
  - for non-matrix jobs: the run directory root

The allowlist MUST include only:
- `git`, `rg`, `cargo`, `just`
- `npm`, `pnpm`, `yarn`, `node`
- `python`, `python3`, `pytest`
- `go`, `make`

`run` steps MUST NOT receive injected preset env variables (preset env injection is reserved for launching ACP agent processes).
`run` steps MUST inherit the `llman` process environment as-is, except for not adding preset env variables.

For matrix-expanded jobs, `run` steps MUST contribute to the report’s terminal command summary (same metrics category as ACP terminal commands).

#### Scenario: Disallowed command is rejected
- **WHEN** a `run` step requests executing a non-allowlisted command (for example, `curl`)
- **THEN** the runner exits non-zero
- **AND** the error explains that the terminal command is not allowed

#### Scenario: `cwd` path traversal is rejected
- **WHEN** a `run` step sets `cwd: "../outside"`
- **THEN** the runner exits non-zero
- **AND** the error explains that path traversal is not allowed

### Requirement: Legacy Playbook Format Is Rejected with Guidance
The runner MUST reject legacy `llman x sdd-eval` playbooks that use the previous fixed-pipeline schema (for example, top-level `version: 1`).

The error MUST be actionable and MUST mention that:
- the playbook format has been replaced by a workflow/jobs/steps DSL, and
- users must update the YAML to the new format.

#### Scenario: Legacy `version: 1` playbook fails with an actionable error
- **WHEN** a user runs `llman x sdd-eval run` with a playbook containing `version: 1`
- **THEN** the command exits non-zero
- **AND** the error clearly indicates the legacy format is not supported
