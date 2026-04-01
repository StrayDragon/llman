# sdd-eval-workflow-dsl Specification (Delta)

## MODIFIED Requirements

### Requirement: Variants Are Addressable by Stable Id and Expandable by Matrix
The playbook MUST define `variants` as a mapping from a stable `variant_id` to a variant configuration.

Each `variant_id` MUST:
- match `^[a-zA-Z][a-zA-Z0-9_-]*$`
- be used as the directory name under `<run_dir>/variants/<variant_id>/`

Each variant configuration MUST define:
- `agent`: an ACP agent definition (kind/preset + optional command/args)

The workflow style MUST be new-style SDD only; legacy styles (for example `sdd-legacy`) MUST NOT be supported, and playbooks MUST NOT include a `variants.<id>.style` field.

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
  - MUST initialize/update SDD templates inside the workspace using new-style SDD only.

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
