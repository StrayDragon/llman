---
name: "llman-sdd-continue"
description: "继续已有 llman SDD 变更，创建下一个缺失工件。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Continue

使用此 skill 继续已有变更，创建下一个缺失的 artifact。

## 步骤
1. 确定 change id：
   - 若用户已提供，直接使用。
   - 否则运行 `llman sdd list --json` 并询问要继续哪个 change。
   - 始终说明："使用变更：<id>"。
2. 阅读变更目录：`llmanspec/changes/<id>/`。
> 阶段判定：用 `llman sdd show <id> --json --type change` 的 `stage` / `readyToImplement` 字段；完整判定表见 llman-sdd-apply。
3. 确定下一个要创建的 artifact（按顺序）：
   1) `proposal.md`
   2) `design.md`（仅当涉及设计权衡时）
   3) `tasks.md`
   4) `llman sdd change start <id>`（或分支已存在时用 `change attach <id>`）——Branch binding
   5) 在**绑定分支**上编辑 live `llmanspec/specs/<capability>/<capability>.feature` 并 commit——Specs landing（无合约变更可设 `skip_specs_landing: true`）
4. 只创建**一个**缺失 artifact（或在绑定分支上做一次 live spec/feature 编辑）。
   - continue 模式**不要**实现应用代码。
   - **不要**创建 `*.feature.delta.toon` 或 `changes/<id>/specs/` 下的文件。
   - **不要**在未 start/attach 前改公共 `llmanspec/specs/**`。
5. 若所有 artifact 已齐全，按 `llman sdd show <id> --json` 建议下一步：
   - `readyToImplement=false` → 先完成 Specs landing（或 `skip_specs_landing`）；**不要**建议 apply
   - `readyToImplement=true` → 实施：`llman-sdd-apply`
   - verify 之后 → 归档：`llman-sdd-archive`
   - 校验：`llman sdd validate <id> --strict --no-interactive`
   - 审查：`llman sdd change diff <id>`（只读）

{{ unit("skills/git-native-flow") }}
> 命令细节用 `llman sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表（r139）。
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
