# feat-sdd-context-bdd-aware

## Why

`tree.rs build_docs()` explicitly drops `MainSpecDoc.scenarios` when building the
pageindex tree, so the retrieval agent can never see the Given/When/Then behavior
details that live in `spec.toon` scenarios. The agent only sees bare MUST/SHALL
statement text, degrading its ability to judge whether a change affects a spec.

Separately, `compute_spec_hash()` only hashes `spec.toon`; editing a derived
`.feature` file does not flip freshness. Although `.feature` is a derived file
(SSOT is `spec.toon`), hashing it defensively catches direct edits.

## What Changes

1. **tree index preserves scenarios**: `build_docs` keeps `feature: true`
   scenarios from the parsed IR into a new `DocNode.scenarios` field (backward
   compatible via `#[serde(default)]`).
2. **staleness hash includes `.feature`**: `compute_spec_hash` hashes
   `*.feature` alongside `spec.toon` so derived-file edits trigger staleness.
3. **retrieval tools expose scenarios**: `get_document_structure` lists scenario
   titles under each req; `get_spec_content` returns matching scenarios'
   Given/When/Then full text; `SYSTEM_PROMPT` guides the agent to read scenarios
   for precise behavior judgment.

## Capabilities

- `sdd-context`: pageindex index build + agentic retrieval now scenario-aware.

## Impact

- **tree.json schema**: `DocNode` gains `scenarios: []` (`#[serde(default)]`;
  old indexes load unchanged — scenarios empty until rebuild).
- **Retrieval output**: `get_spec_content` entries gain a `scenarios` array only
  when the spec has scenarios (non-BDD specs unchanged — progressive).
- **Freshness**: adding/editing `.feature` now marks the index stale.
