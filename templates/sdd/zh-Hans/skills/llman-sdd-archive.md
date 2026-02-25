---
name: "llman-sdd-archive"
description: "归档变更并合并增量到 specs。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 归档

使用此 skill 归档已完成的变更。

## 步骤
1. 确认变更已部署或已接受。
2. 运行 `llman sdd archive run <change-id>`（或兼容入口 `llman sdd archive <change-id>`）。
3. 仅工具类变更使用 `--skip-specs`。
4. 使用 `--dry-run` 预览操作。
5. 重新执行 `llman sdd validate --strict --no-interactive`。

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
