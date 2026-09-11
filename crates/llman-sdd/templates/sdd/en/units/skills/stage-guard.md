## Stage guard (`stage` / `readyToImplement`)

Decide from authoritative JSON (never from vague "complete artifacts" wording):

```bash
llman sdd show <id> --json --type change
```

Read: `stage`, `specsLanded`, `needsSpecsChange`, `readyToImplement`, `gateChecks` (per-item `pass` + one-line `hint` when failing).

| Condition | Action |
|-----------|--------|
| `stage=draft` (proposal.md only) | STOP. Grow to Designed (add design.md) → Planned (add tasks.md) → Branch binding → Specs landing. Draft cannot apply/verify. If proposal+design+tasks exist but stage is still `draft`: tasks-without-design — add design.md first. **Do not** create `changes/<id>/specs/`; **do not** edit live specs on the default branch first. |
| `stage=designed` (proposal + design) | Next: add tasks.md → `planned`. Run `change start` / `attach` (Branch binding) only after planning artifacts are complete. |
| `stage=planned` (proposal + design + tasks) | STOP until binding: run `change start` / `attach` (Branch binding) → `full`. |
| `stage=full` and `readyToImplement=false` | STOP. Finish Specs landing on the **bound branch** (edit `llmanspec/specs/**` and commit), or set `needs_specs_change: false`. **Do not** re-run `change start`. If specs on the bound branch were lost → checkout/recreate + `attach --force` if needed. |
| `readyToImplement=true` | Pass apply/verify prerequisites. `changes/<id>/specs/` is expected to be **absent** — do not treat as missing. |
