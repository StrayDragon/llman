# Design: Remove future-planning Concept

## Rationale

The `future.md` change artifact was designed as a lightweight way to track deferred items
and branch options. In practice, it created maintenance debt:

1. **Out of sight, out of mind**: Once written to `future.md`, items were never reviewed again.
2. **False planning signal**: Skills instructed agents to classify items as `now|later|drop`,
   but `later` and `drop` entries accumulated without actionable triggers.
3. **Template complexity**: The `future-planning.md` unit was included in 5 skill templates
   (propose, new-change, ff, continue, explore) × 2 locales = 10 inclusions, all rendering
   the same guidance that was rarely useful.

## Removed Artifacts

| Artifact | Reason |
|---|---|
| `future-planning.md` unit files (en/zh-Hans) | Core of the misleading guidance |
| `sdd-future-changes` spec | Contract defining future.md as a managed artifact |
| `sdd-workflow` r23 | Mandated "future-to-execution" guidance in skills |
| `future.md` in active changes | Content merged into design.md |

## Added Hook: Scope Integrity Check

In propose/apply preflight, agents now check that every path in each spec's `valid_scope`
exists on disk. This prevents the same class of problem (orphaned references) from occurring
with scopes.

## Migration

Two active changes had `future.md` files with deferred-item notes. Their content was
appended to the respective `design.md` files under a "## 延期实现记录" section before
deleting `future.md`. No information was lost.
