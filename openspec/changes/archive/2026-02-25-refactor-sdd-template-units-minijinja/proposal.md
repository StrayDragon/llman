## Why

Current SDD templates are composed through `{{region: ...}}` expansion plus a large hardcoded embedded-template mapping, which creates high cognitive load and surprising maintenance behavior for contributors. We need a more explicit, modular composition model so maintainers can safely evolve prompts without understanding hidden coupling.

## What Changes

- Introduce a template-unit architecture for SDD prompts where reusable prompt fragments are split into independent, discoverable files.
- Add MiniJinja-based rendering/injection orchestration for SDD template generation to replace region-based coupling in core SDD prompt composition.
- Define deterministic rendering and validation rules (missing unit behavior, duplicate key behavior, locale parity expectations, and output stability).
- Migrate existing SDD skill/spec-driven templates to the new unit injection model while preserving observable workflow behavior.
- Update tests and checks so maintainers can validate template-unit integrity and rendered output consistency.

## Capabilities

### New Capabilities
- `sdd-template-units-and-jinja`: Manage SDD prompt composition through independent template units and MiniJinja injection with explicit contracts.

### Modified Capabilities
- `sdd-workflow`: Update SDD workflow requirements to cover MiniJinja-driven template composition and compatibility expectations.
- `sdd-structured-skill-prompts`: Update structured prompt requirements to require unitized composition and maintain self-contained outputs after rendering.

## Impact

- Affected code: `src/sdd/project/templates.rs`, `src/sdd/project/regions.rs` (or successor module), SDD template loading/rendering path, and update-skills generation path.
- Affected assets: `templates/sdd/{en,zh-Hans}/skills/**` and `templates/sdd/{en,zh-Hans}/spec-driven/**` with new unit files.
- Affected tests/checks: SDD integration tests and template consistency checks.
- Dependencies: existing `minijinja` crate will be used as the rendering engine for unit injection.
