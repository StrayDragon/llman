Validation fixes (single-track feature-as-spec):

1) Missing header comments (`missing `# capability:`` header comment`):
Every capability `.feature` (`llmanspec/specs/<capability>.feature` or `llmanspec/specs/<capability>/<capability>.feature`) MUST start with:
```
# language: zh-CN
# capability: <capability>
# purpose: One-line overview.
# scope: src/
```

2) Tag grammar (`@human constraint scenario must carry an @req:<req_id> tag` / `orphan acceptance scenario`):
- Rules: `@req:<id> @human` — statement in the scenario description (MUST/SHALL required).
- Acceptance: `@executable` + at least one `@req:<id>` linking a rule.
- `@manual` requires `@human`. Never combine `@human` with `@executable`.

3) Legacy `spec.toon` present (`legacy spec.toon found ... run ... toon2features`):
Run `llman sdd project migrate --kind toon2features --yes`, review the diff, commit.

Git-native guardrail:
- **Branch binding** → **Specs landing**: first `change start` / `attach`, then edit live `.feature` files on the bound non-default branch and commit.
- Locked rules: modifying/removing existing `@human` scenarios fails the gate unless the proposal frontmatter `rules_touched` lists the edited req-ids. Confirmation: interactive finalize asks one y/n and writes `rules_touched`; `--yes` acknowledges ONLY `@agent`-marked rules (audit in `agent_acked`); `rules_edit_acked` is removed (r135/q9).
- Apply requires `readyToImplement=true` (or `needs_specs_change: false`). Close-out prefers `change finalize`.
- Do not use `change delta` / solidify / `*.feature.delta.toon`.
