<!-- llman-template-version: 1 -->
## Future-to-Execution Planning
- Treat `llmanspec/changes/<id>/future.md` as a candidate backlog, not passive notes.
- Review `Deferred Items`, `Branch Options`, and `Triggers to Reopen`; classify each item as:
  - `now` (must be converted into executable work now)
  - `later` (keep in future.md with explicit trigger/signal)
  - `drop` (remove or mark rejected with rationale)
- For each `now` item, propose a concrete landing path:
  - follow-up change id (`add-...`, `update-...`, `refactor-...`)
  - affected capability/spec path
  - first executable action (`/llman-sdd:new`, `/llman-sdd:continue`, `/llman-sdd:ff`, or `llman-sdd-apply`)
- Keep traceability: reference source future item in the new proposal/design/tasks notes.
- When uncertainty is high, pause and ask before creating new change artifacts.
