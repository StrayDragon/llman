## Canonical Single-Track Feature Contract

Each capability is ONE Gherkin file: `llmanspec/specs/<capability>/<capability>.feature`.
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
- `@human` scenarios are human-owned constraints; their description carries the normative statement verbatim. Modifying/removing them requires `rules_edit_acked: true` in the change proposal frontmatter.
- `@executable` scenarios are runner-bound acceptance; they link rules via `@req:<req_id>`.
- Coverage tiers: enforced (has acceptance) / manual (`@manual`) / pending. `list --specs` reports all three.
- Scenarios MUST stay top-level: `Rule:` blocks are rejected (the runner skips them silently).
