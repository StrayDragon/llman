---
name: "llman-sdd-onboard"
description: "了解 llman SDD 工作流并完成项目入门。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 入门

使用此 skill 让你快速了解 llman SDD 工作流。

## 步骤
1. 阅读 `llmanspec/AGENTS.md` 与 `llmanspec/project.md`。
2. 查看当前的变更与 specs。
3. 按照 提案 -> 实施 -> 归档 的流程推进。

{{ unit("skills/sdd-commands") }}

## 备注
- `llmanspec/config.yaml` 控制 locale 与 skills 路径。
- locale 仅影响模板与 skills，CLI 仍为英文。
- 使用 `llman sdd-legacy update-skills` 刷新技能。

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
