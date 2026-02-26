## Context

The current SDD pipeline is Markdown-first, with structured protocol sections embedded in generated skills and templates. This worked for initial rollout, but it now has three scaling problems:

1. Prompt governance for ethics-critical tasks is distributed and weakly typed.
2. Template evolution is difficult to measure because old/new quality cannot be compared with a standard A/B workflow.
3. We need to keep legacy behavior available for users while making the improved style the default.

This change introduces an ISON-first pipeline for SDD templates/skills and for runtime spec semantics (`spec.md` parsing, validation, and archive merge), plus a dual-track compatibility model and A/B evaluation workflow.

## Goals / Non-Goals

**Goals:**
- Make ISON the primary source format for SDD template logic.
- Keep rendered Markdown outputs for compatibility with downstream tools.
- Make ISON container payload the canonical semantic source for `llmanspec/specs/**/spec.md` and `llmanspec/changes/**/specs/**/spec.md`.
- Add enforceable ethics governance fields to structured prompt protocol.
- Keep a frozen legacy template track available via explicit selection.
- Add an A/B mechanism to compare legacy/new style quality and safety before and after rollout.
- Set new style as the default for SDD flows.

**Non-Goals:**
- No migration of repository `openspec/specs/**` canonical files into ISON.
- No full rewrite of non-SDD prompt systems.
- No removal of legacy style in this change.

## Decisions

### 1) Dual-track template layout with default-new routing
- Keep `templates/sdd/**` as the new track.
- Add `templates/sdd-legacy/**` as frozen old track.
- Add style routing in SDD commands so default is `new`, and `legacy` is explicit.
- Rationale: explicit paths make maintenance and user communication clear.

### 2) ISON-first source with Markdown rendering
- New track templates are authored in ISON source files and rendered to Markdown outputs consumed by skills and instructions.
- Validation pipeline checks ISON structural validity before rendering.
- Rationale: typed structure and lower entropy improve consistency and reviewability.

### 3) Strong ethics governance in structured protocol
- Introduce required governance fields for risk level, prohibited actions, required evidence, refusal contract, and escalation policy.
- Enforce these fields in validation for new style.
- Rationale: ethics constraints become explicit and testable instead of implicit prose.

### 4) Built-in A/B evaluation workflow
- Add command-level support to run old/new style evaluation on a shared scenario set and emit a report.
- Prioritize quality/safety metrics over token/latency metrics.
- Rationale: rollout decisions need measurable evidence, not only preference.

### 5) Backward compatibility posture
- Keep legacy generation paths available and stable.
- Do not auto-delete or auto-migrate legacy user content.
- Rationale: safe adoption path for teams with existing workflows.

### 6) ISON container contract for specs and delta specs
- `llmanspec/specs/<capability>/spec.md` keeps frontmatter plus one ` ```ison ` block as canonical semantic body.
- `llmanspec/changes/<change>/specs/<capability>/spec.md` uses the same container pattern with `llman.sdd.delta` payload and `ops[]`.
- Runtime check/merge no longer interprets Markdown heading hierarchy (`##/###/####`) as semantic source.
- Rationale: structured merge keys (`req_id`, `scenario.id`) are deterministic and mutation-friendly.

### 7) One-shot migration to ISON semantic engine
- Migrate all active SDD spec/delta files to ISON container payload before enforcing runtime check/merge.
- New writes (archive merge output) always emit ISON container payload.
- Rationale: avoid long-lived dual-parser complexity and remove ambiguity in behavioral diffs.

## Risks / Trade-offs

- [Risk] Dual-track templates increase maintenance overhead.
  - Mitigation: freeze legacy track and evolve only new track.

- [Risk] New validation gates may initially fail existing assumptions.
  - Mitigation: scope strict enforcement to new style and document migration steps.

- [Risk] A/B scoring can be gamed by narrow test cases.
  - Mitigation: keep scenario sets versioned and include safety-focused adversarial cases.

- [Risk] Additional CLI flags can increase UX complexity.
  - Mitigation: default behavior remains simple (`new`), legacy/eval are explicit opt-ins.

- [Risk] One-shot migration can fail on malformed historical Markdown structures.
  - Mitigation: provide deterministic migration tooling, dry-run report, and block switch until migration validation passes.

- [Risk] External ISON SDK maturity may be insufficient.
  - Mitigation: adopt SDK-first evaluation gate, keep internal fallback parser with equivalent AST contract.

## Migration Plan

1. Add new capabilities/specs for ISON pipeline, legacy compatibility, and A/B evaluation.
2. Implement style-routing flags and defaults in SDD commands.
3. Add new/legacy template discovery and generation paths.
4. Add ethics governance enforcement for new style validation.
5. Add A/B evaluation command/reporting.
6. Refactor runtime spec parser/delta parser/validation/archive merge to ISON container semantics.
7. Execute one-shot migration for existing SDD specs and delta specs.
8. Expand integration tests for default-new, explicit-legacy, evaluation outputs, and ISON spec merge semantics.

Rollback strategy:
- If new style prompt generation fails, users can run explicit legacy style generation and validation while fixes are prepared.
- For runtime spec merge issues, keep a short-term feature gate to restore previous release behavior while migration data is preserved.

## Open Questions

- Should future change work add per-tool weighting in A/B scoring (e.g., Codex vs Claude)?
- Should we expose a machine-readable score threshold policy in config, or keep it command-level first?
