## Context

We want an experimental evaluation “runner” to compare outcomes across:

- SDD workflow styles: `sdd` (new) vs `sdd-legacy` (legacy templates / guidance)
- ACP agents: Claude Code ACP and Codex ACP

Today, doing this comparison is manual and non-repeatable. We also need to reuse the existing account/group systems (`llman x cc`, `llman x codex`) while ensuring secrets never leak to playbooks, logs, or run artifacts.

This feature is explicitly experimental: we prioritize a usable skeleton with strong safety defaults, and we cap “SDD loop” execution by a configured iteration limit (no auto-completion detection in v1).

## Goals / Non-Goals

**Goals:**
- Provide `llman x sdd-eval` as a playbook-driven pipeline runner.
- Create isolated per-variant workspaces under `<project>/.llman/sdd-eval/runs/<run_id>/...`.
- Launch ACP agents using presets sourced from existing `llman x cc` / `llman x codex` configs.
- Enforce strict sandboxing: file ops + terminal commands constrained to the workspace root.
- Produce objective, comparable run artifacts + reports; optionally add AI judge scoring.
- Provide human scoring export + import (offline workflow).

**Non-Goals:**
- No attempt to infer “task completion” automatically; loop stops at max iterations.
- No attempt to fully emulate a rich editor UI; only minimal ACP client capabilities.
- No guarantee to support every possible ACP agent feature in v1; focus on the subset needed for Claude Code / Codex evaluation flows.

## Decisions

### Store playbooks and runs under project-local `.llman/`
We store artifacts under `.llman/sdd-eval/` because evaluation is tied to a specific repo/task context and should be easy to share or keep local without touching global config.

### Use `agent-client-protocol` Rust SDK to implement an ACP client
Implement a small ACP client wrapper that spawns the agent process (e.g. `claude-agent-acp`, `codex-acp`) and talks over stdio.

We keep the implementation modular:
- `src/x/sdd_eval/playbook.rs`: YAML structs + validation + resolution of defaults.
- `src/x/sdd_eval/run.rs`: run directory + per-variant workspace lifecycle.
- `src/x/sdd_eval/acp_client.rs`: ACP transport + request routing.
- `src/x/sdd_eval/report.rs`: metrics aggregation + report generation.

### Preset resolution reuses existing config file formats
We do not invent a new secrets store. The pipeline reads:
- Claude Code preset env vars from `claude-code.toml` (same structure as `llman x cc`)
- Codex provider env vars from `codex.toml` (same structure as `llman x codex`)

The playbook only stores preset identifiers (group/provider name). The runner injects resolved env vars into the spawned agent process without printing values.

### Secrets redaction is mandatory for all run outputs
We treat the entire run directory as potentially shareable. Therefore:
- We never serialize env var values into run manifests.
- We never echo env values to stdout/stderr.
- When we log agent/terminal output, we apply a best-effort redaction pass for known secret values that exist in memory (values from presets and `OPENAI_API_KEY`).

### Terminal command execution is allowlisted and workspace-scoped
ACP agents frequently use terminal commands to run tests or package managers. We implement:
- A default deny-by-path policy (no `..`, no absolute path outside workspace).
- A conservative allowlist for commands in v1, with the ability to expand via playbook flags later.

This reduces blast radius if an agent attempts dangerous commands.

### Testing strategy uses a fake ACP agent binary
We implement a small fake ACP agent used only in tests to validate:
- run directory creation and manifests
- sandbox enforcement (path traversal rejected)
- no secrets are written to artifacts

Tests use `TempDir` and `LLMAN_CONFIG_DIR=./artifacts/testing_config_home` patterns to avoid touching real user config.

## Risks / Trade-offs

- **ACP surface area is large** → Start with a minimal subset of requests used in our pipeline; expand as we observe real agents.
- **Secret leakage via agent output** → Best-effort redaction; additionally document that users must avoid echoing secrets in prompts.
- **Workspace copying can be expensive** → v1 uses straightforward copy; can optimize later (git worktree, hardlinks).
- **Command allowlist too strict** → Provide clear errors and a documented escape hatch (future: per-playbook allowlist overrides).

## Migration Plan

- This is a new experimental command, so there is no migration for existing users.
- Artifacts are stored under `.llman/` in the project; removing the directory fully uninstalls run state.

## Open Questions

- What is the minimal ACP request set required for Claude Code ACP and Codex ACP in practice?
- Should we add a stable on-disk schema version for run manifests to enable long-term compatibility?
- Should codex presets map to “provider” or should we introduce an explicit “account group” abstraction?

