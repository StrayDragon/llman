## Context

Current SDD prompt composition relies on `{{region: path#name}}` markers and parser logic in `src/sdd/project/regions.rs`. This approach has three maintainability issues:
- coupling is implicit across many files and hidden in region marker strings,
- onboarding requires understanding marker syntax plus file-type specific extraction rules,
- template embedding is centralized in a very large `embedded_template` match, which is hard to review and evolve.

The change aims to move to explicit template units and MiniJinja-based rendering so composition is visible, testable, and easier for maintainers to reason about.

## Goals / Non-Goals

**Goals:**
- Replace region-style composition with MiniJinja-based unit injection for SDD template generation.
- Split reusable prompt fragments into independent unit files with explicit naming and lifecycle.
- Keep rendered SDD outputs self-contained and behaviorally compatible for end users.
- Reduce contributor surprise by replacing hidden cross-file coupling with explicit injection contracts.
- Preserve locale fallback semantics and template parity checks.

**Non-Goals:**
- No full rewrite of all template-related modules beyond SDD prompt/render path.
- No change to user-facing workflow commands (`llman sdd init/update/update-skills/...`) semantics.
- No introduction of remote template sources or dynamic runtime downloads.

## Decisions

### Decision 1: Introduce explicit template units and a render registry
- Create locale-scoped prompt unit files (for example under `templates/sdd/<locale>/units/**`).
- Define a small registry/manifest contract for which units compose each target template.
- Keep ownership clear: workflow templates declare which units they include; units are reusable but independently editable.

Alternatives considered:
- Keep `region` and improve docs only: rejected; does not solve hidden coupling or parser complexity.
- Keep `region` plus additional linting: rejected; still requires marker syntax and extraction logic.

### Decision 2: Use MiniJinja as composition engine for SDD template rendering
- Use the existing `minijinja` dependency to render templates with unit includes/injection.
- Enforce strict missing-variable/missing-unit errors in render path.
- Keep outputs deterministic (stable unit order where relevant) to reduce diff noise.

Alternatives considered:
- Build custom injection parser: rejected; higher maintenance and lower ecosystem familiarity.
- Keep regex-based replacement for placeholders: rejected; brittle for nested composition.

### Decision 3: Migration with compatibility fence and phased cleanup
- Introduce MiniJinja render path first while preserving output parity checks.
- Migrate SDD skills/spec-driven templates to unit composition incrementally.
- Remove/retire `region` extraction only after parity and tests are green.

Alternatives considered:
- Big-bang migration in one step: rejected; high regression risk and harder reviews.

## Risks / Trade-offs

- [Render contract drift] → Add integration assertions for key generated SKILL/templates and run `check-sdd-templates` in QA.
- [Locale divergence during split] → Require en/zh-Hans unit parity checks in template validation.
- [Contributor confusion during transition] → Add concise docs/comments for unit layout and render registry.
- [Behavior regressions in update-skills output] → Add regression tests for archive/future protocol content in generated SKILLs.

## Migration Plan

1. Add unit file layout + registry contract for SDD template composition.
2. Implement MiniJinja render pipeline in SDD template loader.
3. Migrate shared protocol fragments and archive/future guidance to units.
4. Update `update-skills` and project template rendering to use new pipeline.
5. Keep compatibility checks for rendered outputs and run full QA (`just qa`).
6. Remove or deprecate legacy region parser after parity validation.

Rollback strategy:
- Keep changes modular so renderer can fall back to legacy region expansion if critical regression is found before release.
- Revert the renderer-switch commit while keeping unit files if needed.

## Open Questions

- Should we support mixed mode (legacy region + MiniJinja) for one release cycle, or enforce hard cutover once tests pass?
- What is the minimal maintainer-facing documentation location: `docs/` design note vs inline comments in `templates.rs`?
- Do we need a dedicated `just check-sdd-render` command beyond existing template checks?
