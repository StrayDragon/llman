## 1. OpenSpec artifacts

- [x] 1.1 Create `sdd-eval-acp-pipeline` spec (playbook, runs, security, reporting)
- [x] 1.2 Add `design.md` (ACP client, presets, sandboxing, tests)

## 2. CLI surface

- [x] 2.1 Add `llman x sdd-eval` command group with `init/run/report/import-human`
- [x] 2.2 Add help + error messages; ensure non-zero exits on invalid input

## 3. Playbook + run layout

- [x] 3.1 Define YAML playbook schema + validation (variants, iterations, agent presets)
- [x] 3.2 Implement `.llman/sdd-eval/playbooks/` init template generation
- [x] 3.3 Implement run directory creation under `.llman/sdd-eval/runs/<run_id>/` with manifest
- [x] 3.4 Create per-variant workspace initialization for `sdd` vs `sdd-legacy`

## 4. ACP runner (client side)

- [x] 4.1 Add `agent-client-protocol` dependency and minimal stdio transport wrapper
- [x] 4.2 Implement sandboxed FS + terminal handlers (workspace-only, traversal denied)
- [x] 4.3 Implement agent preset resolution from `llman x cc` + `llman x codex` config formats
- [x] 4.4 Add secret redaction for logs/artifacts (never write env values)

## 5. Reporting

- [x] 5.1 Collect objective metrics (iterations, file diffs, command results) per variant
- [x] 5.2 Implement `report` generation under run directory (JSON + Markdown)
- [x] 5.3 Implement optional AI judge scoring via `OPENAI_*` env vars
- [x] 5.4 Implement human scoring export + `import-human` merge

## 6. Tests + verification

- [x] 6.1 Add fake ACP agent for integration tests (no external installs)
- [x] 6.2 Add integration tests for run layout, sandbox rejection, and no-secret-leak invariant
- [x] 6.3 Run `just fmt`, `just lint`, and `just test`

## 7. OpenSpec archive

- [x] 7.1 Run `openspec validate add-sdd-eval-acp-pipeline --type change`
- [x] 7.2 Archive the change (`openspec archive add-sdd-eval-acp-pipeline`)
- [x] 7.3 Commit and push
