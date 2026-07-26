---
name: "llman-sdd-continue"
description: "继续已有 llman SDD 变更，创建下一个缺失工件。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
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
   - 权威判定阶段：
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     （若无 `jq`，用任意工具从 JSON 解析 `stage` 值。）
   - 若 `stage` 为 `draft`（仅 proposal.md），明确告知用户：「这是 draft 提案。需长大到 `full`（design → tasks → live specs → `change start`）才能实施；draft 不能直接 apply 或 verify。」若已有 proposal+design+tasks 仍是 `draft`，下一步是在非默认 feature 分支上运行 `llman sdd change start <id>`（或 `change attach`）——不要创建 `changes/<id>/specs/`。
3. 确定下一个要创建的 artifact（按顺序）：
   1) `proposal.md`
   2) 在 feature 分支上编辑 live `llmanspec/specs/<capability>/spec.toon`（配置了 `bdd:` 时再加 `*.feature`）
   3) `design.md`（仅当涉及设计权衡时）
   4) `tasks.md`
   5) `llman sdd change start <id>`（或分支已存在时用 `change attach <id>`）
4. 只创建**一个**缺失 artifact（或在分支上做一次 live spec/feature 编辑）。
   - continue 模式**不要**实现应用代码。
   - **不要**创建 `*.feature.delta.toon` 或 `changes/<id>/specs/` 下的文件。
5. 若所有 artifact 已齐全，建议下一步：
   - 实施：`llman-sdd-apply`
   - 校验：`llman sdd validate <id> --strict --no-interactive`
   - 审查：`llman sdd change diff <id>`（只读）
   - 收尾（推荐）：`llman sdd change finalize <id>`（工作区可脏；然后一次 `git commit`）
   - Fallback：`llman sdd change checkpoint <id>`（需干净树）→ `llman sdd change archive <id>`

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
