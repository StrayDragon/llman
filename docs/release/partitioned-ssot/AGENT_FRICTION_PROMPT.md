# Agent friction prompt — Git-native Partitioned SSOT (post-c1250 field notes)

Use this prompt when improving **llman** CLI / skills / validate UX for BDD-on
(Partitioned SSOT). Source: real agent turnaround on a consumer project
(xylitol) while promoting → applying → checkpointing → archiving a live-spec
change.

## Context the agent already has (mostly enough)

- Skills correctly say: live `spec.toon` = constraints; `*.feature` = executable
  GWT + `@req`; attach → checkpoint → docs-only archive → Git merge.
- Templates forbid solidify / `feature_delta` / dual-write of the same scenario id.
- `toon-contract` shows `feature: false` rows in toon vs scenarios only in `.feature`.

What was **missing in practice** was operational sequencing and failure triage,
not the high-level model.

## Concrete friction (fix these)

### 1. Dual-write shape is under-specified for authors

Agents often add `feature: true` **with GWT columns filled** in `spec.toon`
*and* a matching `@req` scenario in `.feature`. Validate then fails with
`dual-write: N executable scenario(s) still have GWT in both…`.

**Desired authoring rule (make explicit in skills + error hint):**

| Artifact | Executable scenario (`@req` / harness) | Doc-only scenario |
|---|---|---|
| `spec.toon` `scenarios[]` | **MUST NOT** appear (or only `scenarios[0]:` if none) | `feature: false` + GWT ok |
| `*.feature` | **only** place for executable GWT | n/a |

Do **not** put `feature: true` rows in toon at all under Partitioned SSOT.
Requirement statements live in toon; examples live in `.feature`.

### 2. Checkpoint dirties the tree; archive needs a second commit

`llman sdd change checkpoint <id>` writes `checkpointed` / `checkpoint_sha` into
`proposal.md`, then `change archive` refuses dirty tree.

**Desired skill/CLI guidance:**

```text
commit live specs + code
→ checkpoint   # mutates proposal frontmatter
→ commit checkpoint metadata
→ archive      # moves change docs only
→ commit archive rename
```

Optionally: checkpoint could stash/auto-commit metadata, or archive could allow
dirty **only** if the sole diff is that change’s proposal checkpoint fields.

### 3. Failure attribution is too weak / too expensive

`validate --specs` reported `Totals: 68 passed, 1 failed` without naming the
failing item in the filtered agent view; full run also executes BDD (~minutes).

**Desired UX:**

1. Totals line **MUST** list failing item ids (e.g. `FAIL package-ai-bridge/dual-write`).
2. Skills: when diagnosing structural gates, prefer
   `llman sdd validate <cap> --strict --no-check` (or `--specs --no-check`)
   **before** a full BDD-bearing validate.
3. Dual-write errors should print the offending `(req_id, scenario.id)` pairs.

### 4. Flag inconsistency

Agents habitually pass `--no-interactive` (from other subcommands).
`change checkpoint` rejects unknown `--no-interactive`.

**Desired:** accept and ignore `--no-interactive` on checkpoint/archive for
skill uniformity, or document the exact flag matrix in `sdd-commands` unit.

### 5. Depends-on vs archive naming

Consumer proposals keep `depends_on: [c1250-…]` after archive renames the
directory to `archive/YYYY-MM-DD-c1250-…`. Confirm validate resolves archived
ids; if not, document the expected frontmatter update in archive skill.

## Non-goals

- Do not restore solidify / feature_delta.
- Do not require agents to run `project migrate --kind partitioned` on every
  change (only when dual-write debt exists).

## Done when (for an llman improvement change)

- [ ] Skills (zh + en): dual-write table + checkpoint→commit→archive sequence
- [ ] Validate: named failures + dual-write scenario ids; `--no-check` path in skills
- [ ] Checkpoint accepts `--no-interactive` or docs list exact flags
- [ ] Optional: archive tolerates checkpoint-only dirty proposal, or checkpoint
      prints “commit proposal.md before archive”
