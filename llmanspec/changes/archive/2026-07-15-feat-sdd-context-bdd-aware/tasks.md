# Tasks

> Design decisions documented in `design.md`.

## Implementation

- [x] 1. `src/sdd/context/tree.rs`: add `ScenarioNode` struct; add `#[serde(default)] scenarios: Vec<ScenarioNode>` to `DocNode`; `build_docs` keeps `feature: true` scenarios from `MainSpecDoc.scenarios` (TREE_VERSION unchanged).
- [x] 2. `src/sdd/context/index.rs`: `compute_spec_hash` collects `*.feature` per spec dir alongside `spec.toon`, sorts, and hashes all together.
- [x] 3. `src/sdd/context/retrieve.rs`: `get_document_structure` lists scenario `{id}` under each req; `get_spec_content` returns matching scenarios' given/when/then; `SYSTEM_PROMPT` adds guidance to read scenarios for behavior judgment.
- [x] 4. Unit tests:
  - tree.rs: `test_build_docs_preserves_scenarios`, `test_build_docs_drops_feature_false`, `test_docnode_loads_without_scenarios_field` (backward compat).
  - retrieve.rs: `test_get_document_structure_includes_scenario_titles`, `test_get_spec_content_includes_scenario_full_text`, `test_get_spec_content_no_scenarios_legacy`.
  - index.rs: `test_compute_spec_hash_includes_feature_files`.
- [x] 5. Run gate: `just fmt && just lint && just test`.

## Validation

- [x] 6. `just run sdd validate feat-sdd-context-bdd-aware --strict --no-interactive` passes.

## Solidify (after apply)

- [x] 7. `just run sdd solidify feat-sdd-context-bdd-aware` regenerates `llmanspec/specs/sdd-context/*.feature`.
