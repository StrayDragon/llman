<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/zh-Hans/opsx/archive.md (copied 2026-02-09; adapted for llman) -->

在 llman SDD 中归档一个已完成的 change。

**输入**：可选在 `/opsx:archive` 后指定 change id（例如：`/opsx:archive add-auth`）。如果省略，先从上下文推断；如仍不明确，运行 `llman sdd list --json` 并询问用户要归档哪个 change。

**步骤**

1. **选择 change**

   - 如果提供了 id，直接使用。
   - 否则运行 `llman sdd list --json`，让用户明确选择要归档的 change id。

   **重要**：不要猜。没有明确确认的 id 就不要归档。

2. **（推荐）先校验**

   运行：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

   如果校验失败，先 STOP，并询问用户是先修复 artifacts 还是明确要继续归档。

3. **（可选）Dry run**

   预览将执行的动作：
   ```bash
   llman sdd archive <id> --dry-run
   ```

4. **归档**

   默认行为（推荐）：
   ```bash
   llman sdd archive <id>
   ```

   仅工具类变更：
   ```bash
   llman sdd archive <id> --skip-specs
   ```

   说明：
   - 默认会把 delta specs 合并进 `llmanspec/specs/`（若存在），并将 change 移动到 `llmanspec/changes/archive/YYYY-MM-DD-<id>/`。
   - 如果目标 archive 目录已存在，STOP 并询问用户下一步如何处理。

5. **验证**

   运行：
   ```bash
   llman sdd validate --strict --no-interactive
   ```

{{ unit("workflow/archive-freeze-guidance") }}

**成功输出示例**

```
## Archive Complete

**Change:** <id>
**Archived to:** llmanspec/changes/archive/YYYY-MM-DD-<id>/
```

**护栏**
- 没有明确确认的 change id，不要归档
- 校验失败时默认停止，除非用户明确选择继续
- 不确定时优先用 `--dry-run` 预览

{{ unit("skills/structured-protocol") }}
