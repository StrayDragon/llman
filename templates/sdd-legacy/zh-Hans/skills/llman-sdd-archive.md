---
name: "llman-sdd-archive"
description: "归档单个或多个变更，并将增量合并到 specs。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 归档

使用此 skill 归档已完成的变更。

## 步骤
1. 确认每个目标 change 都已接受或已部署。
2. 确定目标 ID：
   - 单个模式：一个 `<change-id>`。
   - 批量模式：多个 ID（来自用户输入或 `llman sdd list --json`）。
3. 先逐个校验：`llman sdd validate <id> --strict --no-interactive`。
4. 可选逐个预览归档：`llman sdd archive <id> --dry-run`。
5. 按顺序执行归档：
   - 默认：`llman sdd archive run <id>`（或 `llman sdd archive <id>`）
   - 仅工具类变更：`llman sdd archive run <id> --skip-specs`
   - 任一失败立即停止，并报告剩余未处理 ID。
6. 全部结束后执行一次全量校验：`llman sdd validate --strict --no-interactive`。

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
