## Why
SDD templates live in multiple locales, so it is easy to miss translation updates without a shared version marker. The repo also contains an unused duplicate spec-driven template directory.

## What Changes
- Add per-template version metadata to SDD locale templates.
- Add a maintainer check script and `just check-sdd-templates` command to validate versions and locale parity.
- Remove the unused `templates/sdd/spec-driven/` duplicate directory.

## Impact
- Affected templates: `templates/sdd/en/**`, `templates/sdd/zh-Hans/**`.
- Tooling: `justfile`, new script under `scripts/`.
- Spec: `openspec/specs/sdd-workflow/spec.md`.
