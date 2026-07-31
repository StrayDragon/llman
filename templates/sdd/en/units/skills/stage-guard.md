## Stage guard (`stage` / `readyToImplement`)

Decide from authoritative JSON (never from vague "complete artifacts" wording):

```bash
llman sdd show <id> --json --type change
```

Read: `stage`, `specsLanded`, `skipSpecsLanding`, `readyToImplement`.

| Condition | Action |
|-----------|--------|
| `stage=draft` (proposal.md only) | STOP. Grow to Designed (proposal + tasks; design as needed) → Branch binding → Specs landing. Draft cannot apply/verify. If proposal+design+tasks exist but stage is still `draft`: not started/attached — run `change start` on a clean default branch, or create a branch then `change attach`. **Do not** create `changes/<id>/specs/`; **do not** edit live specs on the default branch first. |
| `stage=designed` | STOP. Run `change start` / `attach` (Branch binding) first. |
| `stage=full` and `readyToImplement=false` | STOP. Finish Specs landing on the **bound branch** (edit `llmanspec/specs/**` and commit), or set `skip_specs_landing`. **Do not** re-run `change start`. If specs on the bound branch were lost → checkout/recreate + `attach --force` if needed. |
| `readyToImplement=true` | Pass apply/verify prerequisites. `changes/<id>/specs/` is expected to be **absent** — do not treat as missing. |
