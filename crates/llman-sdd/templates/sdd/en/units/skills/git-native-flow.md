## Git-native lifecycle (full diagram)

Do not conflate two layers: the **Git-native lifecycle** (Branch binding → Specs landing → `readyToImplement`) vs **skill navigation** (explore→propose→apply→verify→archive). Specs landing is **not** a separate skill.

```mermaid
flowchart TB
  subgraph main_ok["OK briefly on default branch"]
    A["change new → Draft<br/>proposal.md only"]
    B["Fill design + tasks → Designed"]
  end

  subgraph gate_start["Branch binding"]
    C{"Clean tree<br/>and on default branch?"}
    D["change start<br/>create sdd/&lt;id&gt; + write branch/base_sha"]
    E["or manual checkout -b<br/>then change attach"]
  end

  subgraph specs_only["Only on this change branch"]
    F["Edit live llmanspec/specs/** (.feature)"]
    G["commit → Specs landing<br/>base...HEAD includes specs paths"]
  end

  subgraph implement["Implement"]
    H["apply: code per tasks<br/>may keep editing specs"]
    I["verify"]
    J["finalize / archive<br/>ff-merge → specs first hit default branch"]
  end

  A --> B --> C
  C -->|yes| D --> F
  C -->|already on feature| E --> F
  F --> G --> H --> I --> J
```

Hard rules:
1. **First** `change start` / `attach` (Branch binding) to enter Full; **then** edit `llmanspec/specs/**` on the bound non-default branch and commit (Specs landing).
2. For changes with no live contract edits, set frontmatter `skip_specs_landing: true`. Enter apply only when `llman sdd show <id> --json` has `readyToImplement=true` (`Full ∧ (specsLanded ∨ skip)`).
3. **Do not** commit live specs to the default branch just to satisfy the clean-tree gate; if already attached, do not re-run `start`.
