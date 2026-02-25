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
6. 当 archive 目录增长过大时，执行冷备维护：
   - 预览冻结候选：`llman sdd archive freeze --dry-run`
   - 冻结旧归档：`llman sdd archive freeze --before <YYYY-MM-DD> --keep-recent <N>`
   - 需要恢复时：`llman sdd archive thaw --change <YYYY-MM-DD-id>`

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}
