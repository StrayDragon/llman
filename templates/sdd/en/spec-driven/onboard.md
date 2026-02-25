<!-- llman-template-version: 1 -->
<!-- source: OpenSpec src/core/templates/skill-templates.ts:getOnboardInstructions (copied 2026-02-09; adapted for llman) -->

Guide the user through their first complete llman SDD workflow cycle. This is a teaching experience—you'll do real work in their codebase while explaining each step.

---

## Preflight

Ensure the repository is initialized for llman SDD:

- If `llmanspec/` does not exist, instruct the user to run:
  ```bash
  llman sdd init
  ```
  Then continue once it exists.

---

## Phase 1: Welcome

Display:

```
## Welcome to llman SDD!

We'll walk through a complete change cycle—from idea to implementation—using a real task in your codebase.

**What we'll do:**
1. Pick a small, real task in your codebase
2. Explore the problem briefly
3. Create a change (the container for our work)
4. Build the artifacts: proposal → specs → design (optional) → tasks
5. Implement the tasks
6. Validate and archive the completed change

**Time:** ~15-30 minutes (depends on task)
```

Pause and ask:
> "Ready to pick a small starter task?"

---

## Phase 2: Task Selection

Scan the codebase for small improvement opportunities, then present 3-4 concrete options.

Examples of what to look for:
- `TODO` / `FIXME` / `HACK`
- Missing validation or error handling
- Small refactors that improve readability
- Tests missing for small pure functions
- Debug artifacts in committed code

If nothing obvious is found, ask the user what small fix or feature they've been meaning to do.

Scope guardrail:
- If the chosen task is too big, propose a smaller slice so we can complete the full workflow.

---

## Phase 3: Create the Change (Artifacts)

1. Pick a kebab-case change id with a verb prefix (`add-`, `update-`, `remove-`, `refactor-`).
2. Create the artifacts in order under `llmanspec/changes/<id>/`:
   - `proposal.md` (why/what/impact)
   - `specs/<capability>/spec.md` (delta requirements + scenarios)
   - `design.md` (only if design tradeoffs matter)
   - `tasks.md` (ordered, small, verifiable tasks)
3. Validate the change:
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

Pause at key transitions (after proposal, after tasks) and ask the user to confirm before moving on.

---

## Phase 4: Implement

1. Read the artifacts you just created.
2. Implement `tasks.md` in order.
3. After completing each task, update its checkbox (`- [ ]` → `- [x]`).
4. If you hit ambiguity or a blocker, pause and ask the user what to do next.

After implementation, validate again:
```bash
llman sdd validate <id> --strict --no-interactive
```

---

## Phase 5: Archive

When the change is accepted/deployed:

```bash
llman sdd archive <id>
```

Then run:
```bash
llman sdd validate --strict --no-interactive
```

---

## Guardrails

- Keep the starter task small enough to finish end-to-end
- Explain decisions briefly; avoid long lectures
- Do not skip validation steps
- Keep edits minimal and scoped to the tasks

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}
