## 1. Template Tracks and Source Layout

- [x] 1.1 Add `templates/sdd-legacy/{en,zh-Hans}/**` as frozen legacy track with parity/version checks.
- [x] 1.2 Add new-track ISON source layout for SDD templates and shared units.
- [x] 1.3 Wire template loader to resolve style (`new` default, `legacy` explicit) and route to the correct source tree.

## 2. ISON Pipeline and Rendering

- [x] 2.1 Implement ISON parsing/validation module for SDD template sources.
- [x] 2.2 Implement deterministic render path from ISON source to Markdown compatibility outputs.
- [x] 2.3 Update `llman sdd update` and `llman sdd update-skills` to use the style-aware ISON/legacy pipeline.

## 3. Ethics Governance Enforcement

- [x] 3.1 Add required ethics governance fields to new-style structured protocol units/templates.
- [x] 3.2 Extend new-style validation so missing governance fields fail with explicit diagnostics.
- [x] 3.3 Keep legacy validation behavior stable for explicit legacy mode.

## 4. CLI Surface and Evaluation Flow

- [x] 4.1 Add style selector flags to `sdd update`, `update-skills`, `show`, and `validate` with default `new` behavior.
- [x] 4.2 Implement an old-vs-new A/B evaluation flow and report output with safety-first scoring order.
- [x] 4.3 Ensure show/validate behavior is deterministic when style is omitted or explicitly set.

## 5. Tests and Documentation

- [x] 5.1 Add/adjust integration tests for default-new routing, explicit legacy routing, and style-aware generation.
- [x] 5.2 Add tests for ethics governance validation failures in new style.
- [x] 5.3 Add tests or snapshots for A/B report shape and metric ordering.
- [x] 5.4 Update docs/help text for default-new behavior, legacy override usage, and evaluation workflow.

## 6. ISON Spec Engine (Check + Merge)

- [x] 6.1 Add shared ISON container parser utilities (code-fence extraction, tolerant parse, frontmatter compose/split).
- [x] 6.2 Refactor `parse_spec` to read semantic fields from ISON payload instead of Markdown headings.
- [x] 6.3 Refactor `parse_delta_spec` to read `ops[]` from ISON payload and map to add/modify/remove/rename operations.
- [x] 6.4 Refactor archive merge to apply delta ops over requirement ids (`req_id`) and emit merged ISON spec payload.
- [x] 6.5 Update spec/delta validation diagnostics and examples to align with ISON structure.
- [x] 6.6 Update integration/unit fixtures from Markdown heading specs to ISON container specs and re-run tests.
- [x] 6.7 Replace temporary Python migration script with native Rust command `llman sdd migrate --to-ison [--dry-run]`.
