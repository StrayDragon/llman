# Partitioned SSOT → unified Git-native migration

## What changed

| Before | After |
|---|---|
| `spec.toon` scenarios (`feature:true`) projected / dual-written with `.feature` | **Partitioned**: `.feature` = harness SSOT; `spec.toon` = constraints + non-executable scenarios |
| `llman sdd solidify` / `change delta` / `change/specs/` TOON deltas | **Removed**. Edit live specs on a feature branch |
| BDD-on vs BDD-off as two lifecycles | **One** Git-native lifecycle; `bdd:` only toggles the validate runner |
| Archive merges TOON deltas (BDD-off) or docs-only (BDD-on) | **Unified**: ff-merge then docs rename; no TOON merge |
| `project migrate --kind partitioned` | **Removed**. Manually clear dual-writes / leftover deltas |

## Unified Git-native loop

1. `llman sdd change new <id>` (draft) → fill proposal / design / tasks (designed)
2. On the **default** branch with a clean tree: `llman sdd change start <id>` (or manually create a feature branch and `change attach`)
3. Edit live `llmanspec/specs/**` (`spec.toon` + `*.feature` when using harnesses) on the feature branch
4. Implement + `llman sdd validate --specs` / `--check` as needed
5. Close-out (prefer one commit):
   ```bash
   llman sdd change finalize <id> --no-check
   git commit   # implementation + frontmatter + archive rename on default after ff-merge
   ```
   Strict fallback: clean tree → `change checkpoint` → commit → `change archive` → commit rename.

`llman sdd change diff <id>` is read-only review/export — never an input to validate/archive.

## Legacy leftovers

| Leftover | Action |
|---|---|
| Active `*.feature.delta.toon` | Materialize into live `.feature`, delete the delta (`validate` / `archive` ERROR until gone) |
| `changes/<id>/specs/` TOON deltas | Move useful requirements into live `llmanspec/specs/**`, delete the change-scoped specs dir |
| Dual-write executable GWT in both toon and `.feature` | Keep GWT only in `.feature` with `@req`; keep constraints in `spec.toon` |

Archived history under `changes/archive/` stays frozen/readable.

## Commands

```bash
llman sdd change start <id> [--worktree]
llman sdd change attach <id> [--force]
llman sdd change checkpoint <id> [--no-check]
llman sdd change finalize <id> [--no-check]
llman sdd change diff <id> [--export-patch path]
llman sdd change archive <id>
llman sdd project migrate --kind spec-md2toon [--dry-run]   # legacy spec.md → spec.toon only
```
