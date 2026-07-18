---
name: "llman-sdd-continue"
description: "继续一个 llman SDD change：创建下一个缺失的 artifact。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Continue

使用此 skill 继续一个已存在的 change，并创建下一个缺失的 artifact。

## 步骤
1. 确定 change id：
   - 如果用户已提供，直接使用。
   - 否则运行 `llman sdd list --json` 并询问用户要继续哪个 change。
   - 始终说明："使用变更：<id>"。
2. 阅读 change 目录：`llmanspec/changes/<id>/`。
   - 权威地确定当前阶段：
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     （若无 `jq`，可用任意工具从 JSON 中解析 `stage` 值。）
   - 若 `stage` 为 `draft`（仅 proposal.md），明确告知用户："这是一个 draft 提案。需要把它长大到 `full`（specs → design → tasks）后才能实现；draft 不能直接被 apply 或 verify。"{% if bdd_enabled %} BDD-on 下，已有 proposal+design+tasks 仍是 `draft` 意味着变更**未 attach** —— 下一步是 `llman sdd change attach <id>`（在非默认 feature 分支上；BDD-on specs 位于分支，**不要**新增 `changes/<id>/specs/`）。{% endif %}
3. 找出下一个需要创建的 artifact（按顺序）：
   1) `proposal.md`
   2) BDD-off：change 下 `specs/<capability>/spec.toon` delta；BDD-on：在 feature 分支上编辑 live `llmanspec/specs/<capability>/spec.toon` + `*.feature`（未绑定时再 `llman sdd change attach <id>`）
   3) `design.md`（仅当需要讨论设计权衡时）
   4) `tasks.md`
4. 在 `llmanspec/changes/<id>/` 下创建且只创建 ONE 个缺失 artifact（或完成一次 BDD-on live spec/feature 编辑）。
   - continue 模式不要实现应用代码。
   - **禁止**创建 `*.feature.delta.toon`（BDD-on 下为遗留迁移阻断项）。
5. 如果所有 artifacts 都已存在，建议下一步：
   - 实施：`llman-sdd-apply`
   - 校验：`llman sdd validate <id> --strict --no-interactive`
   - BDD-on 审查：`llman sdd change diff <id>`（只读）
   - BDD-on 门禁：`llman sdd change checkpoint <id>`（要求干净工作区）
   - 归档（准备好后）：`llman sdd change archive <id>`

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
