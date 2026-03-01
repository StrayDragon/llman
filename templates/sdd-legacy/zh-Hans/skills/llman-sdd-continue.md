---
name: "llman-sdd-continue"
description: "继续一个 llman SDD change：创建下一个缺失的 artifact。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD Continue

使用此 skill 继续一个已存在的 change，并创建下一个缺失的 artifact。

## 步骤
1. 确定 change id：
   - 如果用户已提供，直接使用。
   - 否则运行 `llman sdd-legacy list --json` 并询问用户要继续哪个 change。
   - 始终说明："使用变更：<id>"。
2. 阅读 change 目录：`llmanspec/changes/<id>/`。
3. 找出下一个需要创建的 artifact（按顺序）：
   1) `proposal.md`
   2) `specs/<capability>/spec.md`（每个 capability 一个文件夹）
   3) `design.md`（仅当需要讨论设计权衡时）
   4) `tasks.md`
4. 在 `llmanspec/changes/<id>/` 下创建且只创建 ONE 个缺失 artifact。
   - continue 模式不要实现应用代码。
5. 如果所有 artifacts 都已存在，建议下一步：
   - 实施：`llman-sdd-apply`
   - 校验：`llman sdd-legacy validate <id> --strict --no-interactive`
   - 归档（准备好后）：`llman sdd-legacy archive <id>`

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
