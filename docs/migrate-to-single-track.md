# Downstream migration guide: Partitioned SSOT → single-track feature-as-spec

This is the **migration prompt** for repositories consuming llman SDD with the
legacy `spec.toon` format (BDD-off or Partitioned). Give it to your coding
agent as-is.

---

## Agent prompt

Your project uses an older llman SDD spec format (`spec.toon` + optional
`*.feature`). The installed llman now speaks ONLY the single-track
feature-as-spec format. Migrate the repository:

1. Run `llman sdd project migrate --kind toon2features --yes`.
   - Every capability directory becomes ONE `<capability>.feature`.
   - Legacy `requirements[]` are preserved VERBATIM as `@req:<id> @human`
     rule scenarios (statement text in the scenario description).
   - Previously runner-bound (`@executable`) scenarios are copied byte-faithful.
   - Unbound documentation scenarios and bare-`@req:` blocks are dropped
     (recoverable from git history).
2. Run `llman sdd validate --all --strict --no-check`. Fix any reported issues:
   - missing `# capability:` / `# purpose:` / `# scope:` headers → add them;
   - orphan acceptance (no `@req`) → link to a rule or drop the scenario;
   - duplicate rule statements → merge or differentiate.
3. Review every `@human` scenario diff by hand: these are your constraints and
   they are now LOCKED. From now on, modifying/removing them requires
   `rules_edit_acked: true` in the change proposal frontmatter.
4. Remove any `bdd.bindings` block from `llmanspec/config.yaml` (retired; the
   `@executable` tag IS the binding declaration). Keep/remove the whole `bdd:`
   runner block per whether you want `validate --check` to execute a runner.
5. Commit as one focused commit: `spec(sdd): migrate to single-track feature-as-spec`.

## What changed conceptually

| Before (Partitioned) | After (single-track) |
|---|---|
| `spec.toon` = constraints + note rows | rules live IN the `.feature` as `@human` scenarios |
| `*.feature` = executable harness | same file also holds the rules |
| dual-write gate between two files | no second file → gate removed |
| harness bound/unbound counts | rule tiers: enforced / manual / pending |
| requirement = prose row in toon | rule = locked, hash-pinned scenario |
