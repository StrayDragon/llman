# Design

## Decision 1: scenarios live in the tree, not parsed from `.feature`

`MainSpecDoc` already parses `scenarios: Vec<ScenarioEntry>` (with `feature: bool`)
from `spec.toon`. `build_docs` throws them away. The fix is to keep
`feature: true` scenarios in `DocNode` — no `.feature` parsing needed in the
index path. `.feature` is a derived artifact; the SSOT is `spec.toon`.

## Decision 2: backward compatibility via `#[serde(default)]`, no version bump

Adding `scenarios` to `DocNode` could break loading old `tree.json`. We mark the
new field `#[serde(default)]` so old indexes load with `scenarios = []` and keep
working until the next rebuild. `TREE_VERSION` stays at 1 — no forced rebuild,
true progressive migration.

**Alternative considered**: bump `TREE_VERSION` + force rebuild on mismatch.
Rejected: violates the "old path must keep working" constraint; users with valid
old indexes shouldn't be forced to rebuild.

## Decision 3: hash `.feature` defensively

`.feature` is derived, so hashing `spec.toon` alone is *correct* under SSOT. But
the user chose to include `.feature` in `compute_spec_hash` defensively: if
someone hand-edits a `.feature`, staleness should fire. Cost is negligible (a few
small files per spec).

## Decision 4: two-layer scenario exposure (structure + content)

Mirrors the existing `reqs` pattern: `get_document_structure` shows scenario
titles (cheap, token-saving); `get_spec_content` returns full Given/When/Then on
demand. Scenarios are attached *only when present* — non-BDD specs produce
byte-identical output to before (progressive).

## Risk

- **Retrieval output schema**: `get_spec_content` entries gain a `scenarios`
  array. Downstream JSON consumers that do strict shape checking could break.
  Mitigation: the field appears only when the spec has scenarios, and existing
  non-BDD specs are unaffected.
- **Token cost**: more data per spec read. Mitigation: structure layer shows only
  scenario `{id}`; full text is opt-in via `get_spec_content`.
