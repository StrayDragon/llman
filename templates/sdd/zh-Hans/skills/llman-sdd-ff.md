---
name: "llman-sdd-ff"
description: "Fast-forward：一次性创建规划壳（proposal/design/tasks），再 Branch binding + Specs landing。禁止写入 changes/<id>/specs/。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Fast-Forward (FF)

快速走完 propose 等价路径：规划壳 → Branch binding → Specs landing（至 `readyToImplement=true`）。**不是**旧的 `changes/<id>/specs/` delta 模型。

## 硬约束

- **规划壳**只写在 `llmanspec/changes/<id>/`（proposal/design/tasks）。
- Live 合约只写在绑定分支的 `llmanspec/specs/**`（Specs landing）。
- **禁止**创建 `llmanspec/changes/<id>/specs/` 或 `*.feature.delta.toon`。
- 进入 apply 前须 `readyToImplement=true`。

## 步骤

1. 询问用户：一句话描述、change id（或派生）、受影响 capability、确认最终 id。
2. 确保已 `llman sdd init`（存在 `llmanspec/`）。
3. 若 `llmanspec/changes/<id>/` 已存在：询问补齐或换 id；勿未确认就覆盖。
4. 创建**规划壳**（可短暂在默认分支）：
   - `llman sdd change new <id>`（或手写）→ 充实 `proposal.md`
   - `design.md`（按需）
   - `tasks.md`
5. **Branch binding**：`llman sdd change start <id>`（干净树 + 默认分支）或手动建分支后 `change attach <id>`。
6. **Specs landing**：在绑定分支编辑 live `llmanspec/specs/<capability>/<capability>.feature` 并 commit；无合约变更则 `skip_specs_landing: true`。
7. 校验：`llman sdd validate <id> --strict --no-interactive`。
8. 用 `llman sdd show <id> --json` 确认 `readyToImplement=true` 后，建议 `llman-sdd-apply`（不要在未就绪时建议 apply）。

{{ unit("skills/git-native-flow-brief") }}
> 命令细节用 `llman sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表（r139）。
{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}
