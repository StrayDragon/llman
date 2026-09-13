## Git-native lifecycle (full diagram)

Do not conflate two layers: the **Git-native lifecycle** (Branch binding → Specs landing → `readyToImplement`) vs **skill navigation** (explore→propose→apply→verify→archive). Specs landing is **not** a separate skill.

```mermaid
flowchart TB
  subgraph main_ok["OK briefly on default branch"]
    A["change new → draft<br/>proposal.md only"]
    B1["add design.md → designed"]
    B2["add tasks.md → planned"]
  end

  subgraph gate_start["Branch binding"]
    C{"Clean tree<br/>and on default branch?"}
    D["change start<br/>create sdd/&lt;id&gt; + write branch/base_branch/base_sha"]
    E["or manual checkout -b<br/>then change attach"]
  end

  subgraph specs_only["Only on this change branch"]
    F["Edit live llmanspec/specs/** (.feature)"]
    G["commit → Specs landing<br/>live merge-base...HEAD includes specs paths"]
  end

  subgraph implement["Implement"]
    H["apply: code per tasks<br/>may keep editing specs"]
    I["verify"]
    J["finalize<br/>merge (squash default) → rename → auto commit archive(sdd): &lt;id&gt;<br/>specs first hit default branch"]
  end

  A --> B1 --> B2 --> C
  C -->|yes| D --> F
  C -->|already on feature| E --> F
  F --> G --> H --> I --> J
```

Hard rules:
1. **First** `change start` / `attach` (Branch binding) to enter Full; **then** edit `llmanspec/specs/**` on the bound non-default branch and commit (Specs landing).
2. For changes with no live contract edits, set frontmatter `needs_specs_change: false`. Enter apply only when `llman sdd show <id> --json` has `readyToImplement=true` — `Full ∧` every `gateChecks` item passes (specs-landed = `specsLanded ∨ needs_specs_change=false`; ranges are live merge-bases, stored `base_sha` is audit-only).
3. `change checkpoint` is removed (no mid-flight archive point; `change finalize` does not require a clean tree). Close-out is `llman sdd change finalize <id>`: it auto-commits `archive(sdd): <id>` (impl diff + rename in one commit); `--no-commit` skips the auto commit for manual/CI histories. Commits on the change branch are free (segmented or finalize single-shot).
4. **Do not** commit live specs to the default branch just to satisfy the clean-tree gate; if already attached, do not re-run `start`.
