## Canonical Single-Track Feature Contract

Each capability is ONE Gherkin file: flat `llmanspec/specs/<capability>.feature` (default) or directory `llmanspec/specs/<capability>/` with a same-named main file — pick one layout, both at once is a conflict.
It is the only spec artifact — there is no `spec.toon`.

```gherkin
# language: zh-CN
# capability: sample
# purpose: One-line overview.
# scope: src/

功能: sample

  @req:r1 @human
  场景: Rule title
    System MUST do something.

  @req:r1 @executable
  场景: happy
    假如 a precondition
    当 a trigger happens
    那么 the outcome is observed
```

- Header comments (`# capability:` / `# purpose:` / `# scope:`) are REQUIRED; `scope` drives staleness.
- `@human` scenarios are human-owned constraints; their description carries the normative statement verbatim. Editing/removing them yields a WARNING only (report-only, never blocks a gate) — compare via git branch diff; the legacy lock-ack metadata `rules_touched` / `agent_acked` / `@agent` is removed with no aliases and no compat layer (locked rules are report-only: a warning, never a block).
- `@executable` scenarios are runner-bound acceptance; they link rules via `@req:<req_id>`.
- Coverage tiers: enforced (has acceptance) / manual (`@manual`) / pending. `list --specs` reports all three.
- Scenarios MUST stay top-level: `Rule:` blocks are rejected (the runner skips them silently).
