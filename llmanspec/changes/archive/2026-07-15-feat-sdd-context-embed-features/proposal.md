# feat-sdd-context-embed-features

## Why

BDD-on specs (e.g. `sdd-workflow`) keep only a few summary scenarios in
`spec.toon` (4) while the real behavior details live in `.feature` files
(93 scenarios across 8 files). The previous change (`feat-sdd-context-bdd-aware`)
made the index read `spec.toon` scenarios, but those are sparse for BDD-on specs.
The retrieval agent still cannot see the `.feature` behavior contract.

## What Changes

1. **index rebuild parses `.feature` files**: `index_rebuild` now loads
   `config.yaml` (locale + bdd), derives the Gherkin language, and parses every
   `*.feature` in each spec dir into spec-level scenarios (`req_id = ""`).
2. **merge + dedup**: `.feature` scenarios are merged into the parsed
   `MainSpecDoc.scenarios`, deduplicated by scenario `id` (toon source wins on
   collision). `build_docs` then keeps them as usual.
3. **retrieval exposes spec-level scenarios**: scenarios with empty `req_id`
   (from `.feature`) are surfaced at the spec level in `get_document_structure`
   (a top-level `scenarios` array) and `get_spec_content` (an extra entry with
   `req_id = ""`), so the agent can read `.feature` behavior details.

## Capabilities

- `sdd-context`: index build now embeds `.feature` content; retrieval exposes it.

## Impact

- **index_rebuild**: now requires `config.yaml` (loads locale/bdd). Non-BDD
  projects (no `.feature` files) are unaffected — `discover_features` returns
  empty, merge is a no-op.
- **tree.json**: `DocNode.scenarios` gains spec-level entries (req_id="") for
  BDD-on specs. Backward compatible (old indexes load, gain content on rebuild).
- **retrieval output**: `get_document_structure` gains a top-level `scenarios`
  array; `get_spec_content` gains an extra `req_id: ""` entry — only when
  spec-level scenarios exist (progressive).
