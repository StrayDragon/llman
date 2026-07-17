# Partitioned SSOT / Git-native BDD — release notes

## Highlights

- **Partitioned SSOT** remains: `spec.toon` = constraints + non-executable scenarios; `*.feature` = executable harness GWT (`@req:<req_id>`).
- **Git-native BDD-on**: each change binds to a non-default Git branch + base SHA. Live files on the branch are SSOT; `git diff base...HEAD` is the only delta.
- **`llman sdd solidify` removed** — no projection, stubs, or feature_delta apply path.
- **Pre-merge archive** moves change documentation only; normal Git/PR merge promotes specs to main.
- **BDD-off** unchanged: TOON delta authoring + archive merge; no runner / attach / checkpoint requirement.

## New commands (BDD-on)

| Command | Role |
|---|---|
| `llman sdd change attach <id>` | Bind current feature branch + merge-base SHA |
| `llman sdd change checkpoint <id>` | Clean tree + validate gates; record checkpoint SHA |
| `llman sdd change diff <id>` | Read-only `base...HEAD` review / optional patch export |

## Migration

See [MIGRATION.md](./MIGRATION.md) and [UPGRADE_AGENT_PROMPT.md](./UPGRADE_AGENT_PROMPT.md).

Active `*.feature.delta.toon` files are blockers — convert to live `.feature` edits before archive.
