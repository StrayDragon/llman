# Design

## Decision 1: merge toon + .feature, dedup by id

`.feature` scenarios are parsed and merged into `MainSpecDoc.scenarios` before
`build_docs`. Dedup key is the scenario `id` (the Gherkin `场景:` name). On
collision, the toon source wins (it is the SSOT for req_id binding). This keeps
the index correct for specs like `sdd-context` where toon and .feature describe
the same scenarios.

**Alternative considered**: prefer .feature on collision (treat .feature as
authoritative). Rejected: toon is still the SSOT and carries req_id binding;
preferring .feature would lose that. Once `.feature` fully replaces toon
scenarios in a future migration, dedup naturally becomes a no-op.

## Decision 2: .feature scenarios are spec-level (req_id = "")

Gherkin has no `req_id` concept. Rather than guessing a req binding, `.feature`
scenarios carry `req_id = ""` and are surfaced at the spec level. This is
forward-compatible: as `.feature` replaces more of the toon spec definition,
these spec-level scenarios become the primary behavior contract without needing
a req binding that doesn't exist in the source format.

## Decision 3: config plumbing for Gherkin language

`index_rebuild` now calls `load_required_config` (already imported, already used
by `context_run`) and derives the Gherkin language via the existing public
`locale_to_gherkin_lang(Some(&config.locale), config.bdd.as_ref())`. This mirrors
`solidify_migrate::run`. Non-BDD projects (no bdd section, no .feature files)
are unaffected: `discover_features` returns empty, so no Gherkin parsing happens
even though `lang` is computed.

## Decision 4: retrieval surfaces spec-level scenarios separately

The existing retrieval API filters scenarios by `req_id == r.req_id`. Spec-level
scenarios (req_id="") would be invisible under that filter. Fix:
- `get_document_structure`: add a top-level `scenarios` array (ids only) listing
  spec-level scenarios, parallel to `reqs`.
- `get_spec_content`: after the req-matched entries, append one extra entry
  `{req_id: "", statement: "", scenarios: [...]}` with all spec-level scenarios'
  full Given/When/Then, when any exist.

Both additions are conditional on spec-level scenarios existing — non-BDD specs
produce byte-identical output to before (progressive).

## Decision 5: .feature parse failures are warnings, not fatal

A malformed `.feature` (bad Gherkin) logs a warning and is skipped, mirroring how
`parse_main_spec` failures are handled (`eprintln!("Warning: ...")` then continue).
One bad file must not abort the whole rebuild.

## Risk

- **config dependency**: `index_rebuild` now requires `config.yaml`. This is
  already the case for `context_run` (retrieval), so the rebuild command gains no
  new constraint that retrieval didn't already have.
- **larger tree.json**: BDD-on specs now embed all .feature scenarios (e.g.
  sdd-workflow gains ~93). Acceptable: tree.json is a local index file, and the
  retrieval agent reads scenarios on demand via tools, not all at once.
