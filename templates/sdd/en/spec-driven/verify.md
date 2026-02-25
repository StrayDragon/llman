<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/verify.md (copied 2026-02-09) -->

Verify that the implementation matches the change artifacts (specs, tasks, design) before archiving.

**Input**: Optionally specify a change id after `/opsx:verify` (e.g., `/opsx:verify add-auth`). If omitted, infer from context; if ambiguous, prompt the user to choose.

**Steps**

1. **Select the change**

   If an id is provided, use it. Otherwise:
   - If the conversation clearly references a change id, use it.
   - Else run `llman sdd list --json` and ask the user to pick a change.

2. **Load artifacts**

   Read what exists under `llmanspec/changes/<id>/`:
   - `proposal.md` (if present)
   - `specs/*/spec.md` (all delta specs)
   - `design.md` (if present)
   - `tasks.md` (if present)

3. **Check Completeness**

   - If `tasks.md` exists, list any unchecked tasks (`- [ ]`) as **CRITICAL**.
   - If no delta specs exist, record **CRITICAL**: "No delta specs found under specs/ (cannot verify requirements)."

4. **Check Correctness**

   For each requirement and scenario in delta specs:
   - Find implementation evidence (files/symbols) and note it
   - Flag likely mismatches as **WARNING** with a concrete recommendation
   - If scenarios are untested, recommend adding tests (or explain why tests are out-of-scope)

5. **Check Coherence**

   - If `design.md` exists, verify implementation follows the key decisions. If not, recommend updating code or updating `design.md` to match reality.
   - Check that changes follow repo conventions (structure, naming, error handling).

6. **Produce a short verification report**

   Output:
   - **CRITICAL** issues (must fix before archive)
   - **WARNING** issues (should fix)
   - **SUGGESTION** items (nice to have)

   End with:
   - If CRITICAL exists: suggest `/opsx:apply <id>` to fix them
   - If clean: suggest `/opsx:archive <id>`

**Guardrails**
- Don’t invent evidence — cite file paths and concrete observations
- Keep recommendations small and actionable

{{ unit("skills/structured-protocol") }}
