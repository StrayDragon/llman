## 1. Playbook DSL (Models + Validation + Template)

- [x] 1.1 Replace `PlaybookV1` YAML model with the new workflow/jobs/steps structs (deny unknown fields; no `version` field).
- [x] 1.2 Implement validation for `variants` (map shape, `variant_id` regex, required fields) and workflow basics (`workflow.jobs` required, step kind is `uses` xor `run`).
- [x] 1.3 Add explicit legacy playbook detection (top-level `version`) with an actionable error message.
- [x] 1.4 Update `llman x sdd-eval init` template YAML to the new DSL and include the `yaml-language-server` schema header.

## 2. Workflow Runner (Jobs/Needs/Matrix)

- [x] 2.1 Implement job graph parsing for `needs` (unknown job id + dependency cycles fail fast).
- [x] 2.2 Implement deterministic execution ordering (topological order with YAML order tie-breaker).
- [x] 2.3 Implement `strategy.matrix.variant` expansion and per-variant execution context (default serial).
- [x] 2.4 Implement minimal `${{ ... }}` interpolation for required paths (`matrix.variant`, `variant.*`, `task.*`, `run.*`), and fail loudly on unknown/missing-context paths.
- [x] 2.5 Refactor run creation to always write `<run_dir>/manifest.json` and `<run_dir>/playbook.yaml` and create per-variant `workspace/`+`logs/`+`artifacts/` directories BEFORE executing the workflow.
- [x] 2.6 Refactor run execution to be workflow-driven (no fixed pipeline order in `run.rs`); move project copy, SDD init/update, and ACP loop to built-in actions.

## 3. Built-In Actions

- [x] 3.1 Introduce an action registry for `builtin:sdd-eval/*` and a dispatcher for `uses:` steps.
- [x] 3.2 Implement `builtin:sdd-eval/workspace.prepare` (workspace/logs/artifacts layout + project copy with existing skip rules).
- [x] 3.3 Implement `builtin:sdd-eval/sdd.prepare` (new vs legacy SDD init/update inside the workspace).
- [x] 3.4 Implement `builtin:sdd-eval/acp.sdd-loop` (preset env injection + bounded loop + session log + metrics artifact).
- [x] 3.5 Implement `builtin:sdd-eval/report.generate` (generate report, optional AI judge, human pack export).

## 4. `run:` Step Execution (Sandbox + Allowlist)

- [x] 4.1 Implement `run` steps as single-command argv execution (POSIX shellwords parsing; reject shell operator tokens; no shell).
- [x] 4.2 Support optional `cwd` as a relative path under the sandbox root; enforce sandbox roots (variant workspace vs run_dir).
- [x] 4.3 Enforce the same terminal command allowlist set as ACP (by basename of argv[0]) and ensure `run` steps contribute to the report terminal-command summary.
- [x] 4.4 Capture output with truncation and apply best-effort secret redaction (reuse `SecretSet`).

## 5. JSON Schema (Auto-Generated) + Schema Check

- [x] 5.1 Derive `JsonSchema` for the new playbook structs (and built-in action `with` structs; v1 defines no `with` keys) with English titles/descriptions.
- [x] 5.2 Extend `llman self schema generate` to emit `artifacts/schema/playbooks/en/llman-sdd-eval.schema.json` (and keep existing config schema outputs unchanged).
- [x] 5.3 Extend `llman self schema check` to validate the playbook schema against an internal template instance (no dependency on user playbook files).

## 6. Tests and Regression

- [x] 6.1 Update `tests/sdd_eval_tests.rs` to use the new workflow DSL template.
- [x] 6.2 Add a regression test: legacy `version: 1` playbook is rejected with an actionable error.
- [x] 6.3 Add a regression test for `run:` allowlist rejection (and confirm sandbox paths remain enforced).
- [x] 6.4 Run `just test` (or `cargo +nightly test --all`) to ensure existing sandbox/secret-redaction guarantees remain intact.
