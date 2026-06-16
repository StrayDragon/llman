---
name: "llman-sdd-sync"
description: "手动把 delta specs 同步到主 specs（不归档 change）。"
---

# LLMAN SDD Sync

使用此 skill 将活动 change 的 delta specs 同步到主 specs（**不归档** change）。

这是一个手动、可复现的协议。

## 步骤
1. 确定 change id（不明确时让用户选择）。
   - 始终说明："使用变更：<id>"。
2. 对每个 delta spec：`llmanspec/changes/<id>/specs/<capability>/spec.toon`
   - 阅读 delta
   - 阅读（或创建）主 spec：`llmanspec/specs/<capability>/spec.toon`
   - 按 delta 语义手动应用（add/modify/remove/rename + scenarios），保持主 spec 为独立的 TOON 文档
3. 校验 specs：
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. sync 不负责归档；准备好后执行 `llman sdd archive run <id>`。

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
