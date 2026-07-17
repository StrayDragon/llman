# Partitioned SSOT → Git-native BDD migration

## What changed

| Before | After |
|---|---|
| `spec.toon` scenarios (`feature:true`) projected / dual-written with `.feature` | **Partitioned**: `.feature` = harness SSOT; `spec.toon` = constraints + non-executable scenarios |
| `llman sdd solidify` consistency gate / stubs / feature_delta | **Removed**. Edit live `.feature` on a feature branch |
| Change archives apply `*.feature.delta.toon` | **BDD-on**: archive moves change docs only; Git merge promotes specs |
| Executable changes authored as `feature_delta` | Edit branch-local `llmanspec/specs/**/*.feature` (+ `@req`) |

## BDD-on Git-native loop

1. Create/switch to a **non-default** feature branch
2. Author change docs under `llmanspec/changes/<id>/`
3. Edit live `spec.toon` + `*.feature` on the branch
4. `llman sdd change attach <id>`
5. Implement + `llman sdd validate --specs` / `--check` as needed
6. Clean tree → `llman sdd change checkpoint <id>`
7. `llman sdd change archive <id>` (docs only)
8. Open PR / merge the branch to the default branch

`llman sdd change diff <id>` is read-only review/export — never an input to validate/archive.

## Legacy feature_delta

Active `*.feature.delta.toon` under a change is a **migration blocker** (`validate` / `archive` ERROR). Convert those ops into live `.feature` edits on the feature branch, then delete the delta files.

Archived historical feature_delta under `changes/archive/` remains readable and frozen.

## Commands

```bash
llman sdd project migrate --kind partitioned   # split remaining toon dual-writes
llman sdd change attach <id>
llman sdd change checkpoint <id> [--no-check]
llman sdd change diff <id> [--export-patch path]
llman sdd change archive <id>
```

BDD-off projects keep the classic TOON delta archive path via `llman sdd change delta …` then `llman sdd change archive`, and must not use attach/checkpoint.

Draft a change shell with `llman sdd change new <id>` (both modes).
