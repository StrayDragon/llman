## Git-native 生命周期（权威全图）

勿混淆两层：**Git-native 生命周期**（Branch binding → Specs landing → `readyToImplement`）与 **Skill 导航**（explore→propose→apply→verify→archive）。Specs landing **不是**独立 skill。

```mermaid
flowchart TB
  subgraph main_ok["允许短暂在默认分支"]
    A["change new → draft<br/>仅 proposal.md"]
    B1["补 design.md → designed"]
    B2["补 tasks.md → planned"]
  end

  subgraph gate_start["Branch binding"]
    C{"工作区干净<br/>且在默认分支？"}
    D["change start<br/>建 sdd/&lt;id&gt; + 写 branch/base_branch/base_sha"]
    E["或手动 checkout -b<br/>再 change attach"]
  end

  subgraph specs_only["仅在本 change 分支"]
    F["编辑 live llmanspec/specs/**（.feature）"]
    G["commit → Specs landing<br/>现算 merge-base...HEAD 含 specs 路径"]
  end

  subgraph implement["实现"]
    H["apply：按 tasks 改代码<br/>可继续改 specs"]
    I["verify"]
    J["finalize<br/>合并（squash 缺省）→ rename → 自动提交 archive(sdd): &lt;id&gt;<br/>目标分支才首次合入 specs"]
  end

  A --> B1 --> B2 --> C
  C -->|是| D --> F
  C -->|已在 feature| E --> F
  F --> G --> H --> I --> J
```

硬规则：
1. **先** `change start` / `attach`（Branch binding / 分支绑定）进入 Full；**再**在绑定的非默认分支编辑 `llmanspec/specs/**` 并 commit（Specs landing / 合约落地）。
2. 无 live 合约变更时可设 frontmatter `needs_specs_change: false`。进入 apply 前 `llman sdd show <id> --json` 的 `readyToImplement` 须为 true（`Full ∧ gateChecks 全过`；specs-landed 项 = `specsLanded ∨ needs_specs_change=false`；一切范围 = 现算 merge-base，存储 `base_sha` 仅审计）。
3. `change checkpoint` 已移除（无存档点概念：中途不必存档，`change finalize` 不要求干净树）。收口一律 `llman sdd change finalize <id>`：自动提交 `archive(sdd): <id>`（实现 diff + 改名一次提交）；`--no-commit` 跳过自动提交（CI/手动历史场景）。change 分支上提交自由（分段或 finalize 单次收尾均可）。
4. **禁止**为过干净树门禁把 live specs commit 到默认分支；已 attach 时不要重复 `start`。
