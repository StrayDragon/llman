<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/zh-Hans/opsx/bulk-archive.md (copied 2026-02-09; adapted for llman) -->

在 llman SDD 中批量归档多个已完成的 change。

**输入**：可在 `/opsx:bulk-archive` 后提供多个 change id（用空格分隔）。如果省略，请从活动 changes 中让用户选择。

**步骤**

1. **确定要归档的 change ids**

   运行：
   ```bash
   llman sdd list --json
   ```

   如果没有活动 changes，提示用户并 STOP。

   让用户明确选择要归档的 change ids（1+）。不要猜。

2. **逐个归档（遇到失败即停止）**

   按顺序对每个 id 执行：

   - （推荐）先校验：
     ```bash
     llman sdd validate <id> --strict --no-interactive
     ```
     如果校验失败，STOP 并询问用户是先修复 artifacts 还是明确要继续归档。

   - （可选）预览：
     ```bash
     llman sdd archive <id> --dry-run
     ```

   - 归档：
     ```bash
     llman sdd archive <id>
     ```

     仅工具类变更使用：
     ```bash
     llman sdd archive <id> --skip-specs
     ```

   若任意一次归档失败，STOP 并报告错误（不要继续处理后续 ids）。

3. **最终验证**

   运行：
   ```bash
   llman sdd validate --strict --no-interactive
   ```

**成功输出示例**

```
## Bulk Archive Complete

Archived:
- <id-1>
- <id-2>
```

**护栏**
- 没有明确确认的 change ids，不要归档
- 遇到失败立刻停止并报告
- 归档前优先做校验
