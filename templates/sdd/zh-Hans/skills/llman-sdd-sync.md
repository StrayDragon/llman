---
name: "llman-sdd-sync"
description: "手动把 delta specs 同步到主 specs（不归档 change）。"
metadata:
  llman-template-version: 3
---

# LLMAN SDD Sync

使用此 skill 将活动 change 的 delta specs 同步到主 specs（**不归档** change）。

这是一个手动、可复现的协议。

## 步骤
1. 确定 change id（不明确时让用户选择）。
   - 始终说明："使用变更：<id>"。
2. 对每个 delta spec：`llmanspec/changes/<id>/specs/<capability>/spec.md`
   - 阅读 delta
   - 阅读（或创建）主 spec：`llmanspec/specs/<capability>/spec.md`
   - 按 `table.ops` + `table.op_scenarios` 手动应用（add/modify/remove/rename）
3. 校验 specs：
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. sync 不负责归档；准备好后使用 `/llman-sdd:archive`。

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
