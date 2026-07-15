# Tasks

> Design decisions documented in `design.md`.

## Implementation

- [x] 1. `src/sdd/solidify.rs`: add `pub fn parse_feature_file(path: &Path, lang: &str) -> Result<Vec<ScenarioNode>>` (mirrors `scenario_from_gherkin`, req_id="", GherkinEnv rebuilt per file).
- [x] 2. `src/sdd/context/mod.rs`: `index_rebuild` loads `load_required_config`, derives `lang`; `index_rebuild_pageindex` gains `lang` param, calls `discover_features` + `parse_feature_file`, merges (dedup by id, toon wins) into `MainSpecDoc.scenarios` before `build_docs`; malformed feature → warn + skip.
- [x] 3. `src/sdd/context/retrieve.rs`: `get_document_structure` adds top-level `scenarios` array (ids, req_id=""); `get_spec_content` appends `req_id:""` entry with spec-level scenario full text; both conditional on spec-level scenarios existing.
- [x] 4. Tests:
  - solidify.rs: `test_parse_feature_file_extracts_scenarios`, `test_parse_feature_file_empty_when_no_scenarios`.
  - mod.rs (TempDir): `test_index_rebuild_embeds_feature_scenarios_bdd` (config with bdd + .feature → merged scenarios, req_id empty); `test_index_rebuild_non_bdd_no_features` (config without bdd, no .feature → toon-only, unchanged).
- [x] 5. Run gate: `just fmt && just lint && just test`.

## Validation

- [x] 6. `just run sdd validate feat-sdd-context-embed-features --strict --no-interactive` passes.

## Solidify (after apply)

- [x] 7. `just run sdd solidify feat-sdd-context-embed-features` regenerates feature files.
