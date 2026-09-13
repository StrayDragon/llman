---
name: "llman-sdd-archive"
description: "Archive completed llman SDD changes. Auto-merge back into the fork-point branch (squash by default), then rename change docs to archive/. Use after verify reports all-clear."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Archive

Use this skill to archive completed changes. Prerequisites: verify all-green, and the change already has Branch binding plus Specs landing (or `needs_specs_change: false`; live specs are on the bound branch). `change finalize` **auto-merges** into the fork-point branch (target: `--into` > binding `base_branch` > default branch; method: `--method` > config `sdd.merge_method`, squash by default — feature diff + rename collapse into ONE close-out commit on the target), **renames** change docs to `changes/archive/`, then **auto-commits** `archive(sdd): <change-id>` (impl diff + rename in one commit; `--no-commit` skips). `change checkpoint` is removed (no mid-flight archive point; `change finalize` does not require a clean tree). `git push` / hosting PR are optional.

## Pipeline Position

```mermaid
flowchart LR
    verify["llman-sdd-verify<br/>Verify"] --> archive
    archive["★ llman-sdd-archive ★<br/>Archive (you are here)"]

    style archive fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 You are in the archive phase: the last stop in the Git-native lifecycle.
> 📎 If specs get too large, run `llman-sdd-specs-compact` to compress.

## Hard Constraints

- **Must pass verify phase all-green first**: don't archive changes that haven't passed verification.
- **Must already have Branch binding**: `change start` / `attach` done; otherwise STOP.
- **SSOT validation**: every change must pass `llman sdd validate <id> --strict --no-interactive` before archiving.
- **Don't ask "should I continue?"**: execute the full batch to completion unless you hit an unresolvable error.
- **Close-out MUST NOT default to PR/push**: finalize performs a local merge (squash by default) + rename + one close-out commit (`archive(sdd): <id>`). `git push` / hosting PR are optional — only when the user or project explicitly requires remote review. **Agent MUST NOT** push or open a PR by default on this skill's account.

## Steps

### 0) Preflight
- `git status --porcelain`: confirm working tree changes belong to completed changes.
- If unexpected changes exist, handle them (stash or report).

### 1) Confirm target changes
- Determine target IDs: single or batch (from user input or `llman sdd list --json`).
- Always announce: "Archiving IDs: <id1>, <id2>, ...".
- Confirm each change has passed verify phase all-green.

### 2) Archive one by one
- **Human review checkpoint (before each id is archived, including batches)**: run `llman sdd review --capability <id>`. Exit code zero → continue; non-zero = CRITICAL findings: STOP, fix, re-run; MUST NOT archive with CRITICAL findings open.
- Validate each first: `llman sdd validate <id> --strict --no-interactive`.
- Validation failure → STOP and report; don't skip validation and force archive.
- Optional preview: `llman sdd change archive <id> --dry-run`.
- Execute archive:
  - default: `llman sdd change archive <id>`
  - tooling-only: `llman sdd change archive <id> --skip-specs`
  - **stop immediately on first failure**, report remaining unprocessed IDs.
- **Git-native close-out**:
  - Prerequisites: Branch binding done (`change start` / `attach`); still on the bound branch (or the target branch after the auto merge).
  - `change archive` / `change finalize` run the **auto merge** (target `--into` > binding `base_branch` > default branch; method squash by default or `ff`; if the target is held by another worktree the merge is skipped with an explicit manual command), **then** rename change docs into `changes/archive/` — rename is never rolled back on merge failure and degradation is reported explicitly.
  - Legacy `*.feature.delta.toon` or `spec.toon` under specs is a migration blocker — run `llman sdd project migrate --kind toon2features`.
  - **Default: `change finalize` (one-command close)** — gates → auto merge → docs rename → **auto commit** `archive(sdd): <change-id>` (squash default: impl diff + rename collapse into ONE commit on the target; no manual `git commit` needed; locked-rule edits are a report-only WARNING — warn, never block):
    ```text
    1. Implement live specs + code (working tree may stay dirty; commits on the branch are free — segmented or none)
    2. llman sdd change finalize <id>    # gates + merge (squash default) + rename + auto commit
    3. optional: git commit --amend    # adjust the message; git branch -D <feature>  # after squash the branch is no longer an ancestor; -d gets refused
    ```
    `--no-commit` skips the auto commit (CI / pre-commit-hook conflicts): finalize then leaves the tree dirty and prints the manual `git commit` command. Idempotent retry: a rerun after a failed auto commit detects the already-archived rename and finishes the commit.
  - **Fallback: plain `change archive <id>`** — same merge + rename, no auto commit; requires a clean tree. `checkpointed`/`checkpoint_sha` fields went away with checkpoint (no mid-flight archive point; `change finalize` needs no clean tree) — nothing to write beforehand, and nothing to review for the snapshot (use `change diff` instead).

### 3) Full validation
- After all archives complete: `llman sdd validate --all --strict --no-interactive`.
- Confirm post-archive spec artifacts are consistent.

### 4) Commit guidance
- Finalize auto-committed (`archive(sdd): <id>`); with `--no-commit`, commit manually: `git add -A && git commit -m "archive(sdd): <id1>, <id2>"` (or the archive skill's suggested format).
- Optional: `git branch -D <feature>` after the merge (squash leaves the branch outside main's ancestry). push / hosting PR only when the user or project explicitly requires remote review.
- **Breaking contract changes** (removed/renamed frontmatter field, command, tag, or stage value) MUST ship an upgrade path under `migrations/v<from>-v<to>/` (README prompt + one-shot script, shipped in-repo) — verify it exists before closing the change.
- **Archived `depends_on`**: archive renames the change dir to `archive/YYYY-MM-DD-<id>`, but validate recognizes `depends_on` pointing to archived/frozen ids as INFO (not ERROR), so you do **not** need to manually update other changes' `depends_on` frontmatter after archive.

> 💡 Previous phase `llman-sdd-verify` (passed verification) → this phase completes the loop. If specs grow too large, run `llman-sdd-specs-compact`.

{{ unit("workflow/archive-freeze-guidance") }}

> For command details run `llman sdd <cmd> --help`; the CLI is the command reference — skills embed no command tables.
> "Spec" here = a `.feature` file under this project's `llmanspec/specs/`; run `llman sdd list --specs` or `llman sdd show <capability>`.

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
