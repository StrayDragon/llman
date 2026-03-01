<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/en/llman-sdd/continue.md (copied 2026-02-09) -->

Continue working on a change by creating the next artifact in `llmanspec/changes/<id>/`.

**Input**: Optionally specify a change id after `/llman-sdd:continue` (e.g., `/llman-sdd:continue add-auth`). If omitted, infer from context; if ambiguous, you MUST prompt the user to pick a change.

**Steps**

1. **Select the change**

   If a change id is provided, use it. Otherwise:
   - If the conversation clearly references a single change id, use it.
   - Else run `llman sdd-legacy list --json`, show the top 3–4 most recently modified changes, and ask the user which one to continue.
   Always announce: "Using change: <id>" and how to override (e.g., `/llman-sdd:continue <other>`).

2. **Verify the change exists**

   Confirm the directory exists: `llmanspec/changes/<id>/`.
   - If missing: suggest starting with `/llman-sdd:new <id>` and STOP.

3. **Determine what artifact to create next (spec-driven)**

   Use the default spec-driven ordering:
   1) `proposal.md`
   2) `specs/<capability>/spec.md` (one capability at a time)
   3) `design.md` (recommended when needed; optional)
   4) `tasks.md`

   Decide what’s missing by checking file existence under `llmanspec/changes/<id>/`.

4. **Create exactly ONE artifact**

   - If `proposal.md` is missing: create it (Why / What Changes / Capabilities / Impact).
   - Else if no delta spec exists yet under `specs/*/spec.md`:
     - Ask for the first capability id (kebab-case), OR derive it from the proposal’s Capabilities section.
     - Create `llmanspec/changes/<id>/specs/<capability>/spec.md` using `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` with at least one `#### Scenario:` per requirement.
   - Else if `design.md` is missing and the change seems to need design (multi-system, tricky tradeoffs, breaking changes):
     - Create `design.md` capturing decisions and reasoning.
   - Else if `tasks.md` is missing:
     - Create `tasks.md` as an ordered, checkable list of small, verifiable items (include validation commands).
   - Else:
     - All planning artifacts exist. Suggest `/llman-sdd:apply <id>` to implement or `/llman-sdd:archive <id>` when ready, then STOP.

5. **Suggest validation**

   - If at least one delta spec exists: suggest running `llman sdd-legacy validate <id> --strict --no-interactive`.
   - Otherwise: explain that change validation will fail until a delta spec exists (by design).

**Output**

After each invocation, show:
- Which artifact you created and its path
- What’s next (what remains)
- Prompt: "Run `/llman-sdd:continue <id>` to create the next artifact"

**Guardrails**
- Create ONE artifact per invocation
- Read existing artifacts before writing a new one
- If anything is unclear, ask before creating the artifact
- Do NOT implement application code in continue mode

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
