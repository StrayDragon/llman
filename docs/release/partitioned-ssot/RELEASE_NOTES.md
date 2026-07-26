# Partitioned SSOT / Git-native — release notes

## Highlights

- **Partitioned SSOT** remains: `spec.toon` = constraints + non-executable scenarios; `*.feature` = executable harness GWT (`@req:<req_id>`).
- **Unified Git-native lifecycle** (BDD-on and BDD-off): draft → designed → `change start` (or `attach`) → apply → verify → archive/finalize. Edit live `llmanspec/specs/**` on a feature branch.
- **`bdd:` is runner-only** — it controls whether `validate --check` runs `bdd.run_command`, not the change lifecycle.
- **`llman sdd solidify` / `change delta` / `llman-sdd-sync` removed** — zero compatibility.
- **Archive / finalize**: auto **ff-merge** feature → default branch, then **docs rename** to `changes/archive/YYYY-MM-DD-<id>/` (dirty rename left for one follow-up commit). No TOON delta merge.
- Prefer **`change finalize`** for single-commit close-out (relaxed clean-tree); `checkpoint` + `archive` remains the strict fallback.

## Key commands

| Command | Role |
|---|---|
| `llman sdd change start <id>` | Clean tree on default → create `sdd/<id>` (+ optional `--worktree`) + attach binding |
| `llman sdd change attach <id>` | Bind an existing non-default branch + merge-base SHA |
| `llman sdd change checkpoint <id>` | Clean tree + validate; record `checkpoint_sha = HEAD` |
| `llman sdd change finalize <id>` | Validate + write `checkpoint_sha = base_sha` + ff-merge + docs rename (dirty OK) |
| `llman sdd change diff <id>` | Read-only `base...HEAD` review / optional patch export |
| `llman sdd change archive <id>` | Strict gates → ff-merge → docs rename |

## Migration

See [MIGRATION.md](./MIGRATION.md) and [UPGRADE_AGENT_PROMPT.md](./UPGRADE_AGENT_PROMPT.md).

Active `*.feature.delta.toon` or leftover `changes/<id>/specs/` are blockers — clean them manually (the old `project migrate --kind partitioned` command is removed).
