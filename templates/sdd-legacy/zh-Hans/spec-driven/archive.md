<!-- llman-template-version: 2 -->
<!-- source: llman sdd-legacy workflow templates/zh-Hans/archive.md -->

在 llman SDD 中归档已完成的 changes。

**输入**：可选在 `/llman-sdd:archive` 后提供一个或多个 change id（空格分隔）。如果省略，先从上下文推断；如仍不明确，运行 `llman sdd-legacy list --json` 并让用户选择。

**步骤**

1. **确定目标 IDs**

   - 如果已提供一个或多个 id，按顺序使用。
   - 否则运行 `llman sdd-legacy list --json`，让用户明确选择 IDs。

   **重要**：不要猜。没有明确确认的 IDs 就不要归档。
   始终说明："归档 IDs：<id1>, <id2>, ..."。

2. **（推荐）先逐个校验**

   对每个 id 运行：
   ```bash
   llman sdd-legacy validate <id> --strict --no-interactive
   ```

   若任一校验失败，先 STOP，并询问用户是先修复 artifacts 还是明确继续。

3. **（可选）逐个 Dry run**

   ```bash
   llman sdd-legacy archive <id> --dry-run
   ```

4. **按顺序执行归档**

   默认行为（推荐）：
   ```bash
   llman sdd-legacy archive run <id>
   ```

   仅工具类变更：
   ```bash
   llman sdd-legacy archive run <id> --skip-specs
   ```

   说明：
   - 归档目标按顺序逐个处理。
   - 任一失败立即停止，并报告已完成与待处理 IDs。
   - 成功归档会把 delta specs 合并进 `llmanspec/specs/`（若存在），并移动到 `llmanspec/changes/archive/YYYY-MM-DD-<id>/`。

5. **全部结束后执行一次验证**

   ```bash
   llman sdd-legacy validate --strict --no-interactive
   ```

{{ unit("workflow/archive-freeze-guidance") }}

**成功输出示例**

```
## Archive Complete

**Archived IDs:** <id1>, <id2>, ...
**Archive Root:** llmanspec/changes/archive/
```

**护栏**
- 没有明确确认的 IDs，不要归档
- 校验失败默认停止，除非用户明确选择继续
- 不确定时优先使用 `--dry-run`
- 批量模式下任一失败立即停止，并保留可审计的完成清单

{{ unit("skills/structured-protocol") }}
