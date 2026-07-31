## Git-native sub-flow (Branch binding → Specs landing)

Alongside the main skill chain (explore→propose→apply→verify→archive), the propose phase contains a Git-native sub-flow. Specs landing is **not** a separate skill.

```mermaid
flowchart LR
  designed[Designed_shell] --> bound[change_start_or_attach]
  bound --> landed[Specs_landing]
  landed --> applyReady[readyToImplement]
  applyReady --> applySkill[llman_sdd_apply]
```

Hard rules:
1. **First** `change start` / `attach` (Branch binding) to enter Full; **then** edit `llmanspec/specs/**` on the bound non-default branch and commit (Specs landing).
2. For changes with no live contract edits, set frontmatter `skip_specs_landing: true`. Enter apply only when `llman sdd show <id> --json` has `readyToImplement=true`.
3. **Do not** commit live specs to the default branch just to satisfy the clean-tree gate; if already attached, do not re-run `start`.
