# baselines/

Versioned snapshots of SDD skill templates, used as A/B baselines by
`run-sdd-skill-gate-eval.sh --baseline-skill <file>`.

## How to create a snapshot

Use `git show` to extract a skill template at a specific ref:

```bash
# Snapshot the apply skill as of HEAD~1 (the previous committed version):
git show HEAD~1:templates/sdd/zh-Hans/skills/llman-sdd-apply.md \
  > agentdev/promptfoo/baselines/llman-sdd-apply-HEAD~1.md

# Or pin to a tag / commit sha:
git show v0.0.64:templates/sdd/zh-Hans/skills/llman-sdd-draft.md \
  > agentdev/promptfoo/baselines/llman-sdd-draft-v0.0.64.md
```

## How to use a snapshot

```bash
bash scripts/sdd-skill-gate-eval.sh \
  --skill llman-sdd-apply \
  --baseline-skill agentdev/promptfoo/baselines/llman-sdd-apply-HEAD~1.md \
  --runs 2
```

The runner renders the snapshot into the baseline provider's prompt and the
current workspace template into the candidate provider's prompt, producing a
true A/B comparison in one eval run.

## Management policy

- This directory is **manually / CI managed**. The eval runner does NOT write
  snapshots here automatically (to avoid eval side-effects polluting baselines).
- Commit snapshots you want to keep as durable regression anchors. Treat them
  like test fixtures: reviewed, intentional, version-named.
- Naming convention: `<skill-id>-<ref>.md` (e.g. `llman-sdd-apply-HEAD~1.md`,
  `llman-sdd-draft-v0.0.64.md`).
