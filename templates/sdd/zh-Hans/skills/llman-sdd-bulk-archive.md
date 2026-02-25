---
name: "llman-sdd-bulk-archive"
description: "安全地批量归档多个 llman SDD changes。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD Bulk Archive

使用此 skill 逐个归档多个 changes，并在最后做一次全量校验。

## 步骤
1. 列出活动 changes：`llman sdd list --json`。
2. 让用户明确选择要归档的 change ids（1+）。不要猜。
3. 按顺序处理每个 id（遇到失败立刻停止）：
   - （推荐）校验：`llman sdd validate <id> --strict --no-interactive`
   - （可选）预览：`llman sdd archive <id> --dry-run`
   - 归档：`llman sdd archive <id>`（仅工具类变更使用 `--skip-specs`）
4. 全部成功后运行：
   ```bash
   llman sdd validate --strict --no-interactive
   ```
5. 若 archive 目录过多，可选执行一轮冻结：
   - 预览：`llman sdd archive freeze --dry-run`
   - 冻结：`llman sdd archive freeze --keep-recent <N>`

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
