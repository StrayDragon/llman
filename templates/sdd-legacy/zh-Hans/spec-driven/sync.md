<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/sync.md (copied 2026-02-09; adapted for llman) -->

将活动 change 中的 delta specs 同步到主 specs，**但不归档**该 change。

这是一个手动、可复现的协议：读取 `llmanspec/changes/<id>/specs/` 下的 delta specs，并把变更应用到 `llmanspec/specs/`。

**输入**：可选在 `/llman-sdd:sync` 后指定 change id（例如：`/llman-sdd:sync add-auth`）。如果省略，请从上下文推断；如仍不明确，提示用户选择。

**步骤**

1. **选择 change**

   如果提供了 id，直接使用。否则运行 `llman sdd-legacy list --json` 并让用户选择要同步的 change。
   始终说明："使用变更：<id>"，并告知如何覆盖（例如 `/llman-sdd:sync <other>`）。

2. **查找 delta specs**

   在以下位置查找 delta specs：
   - `llmanspec/changes/<id>/specs/<capability>/spec.md`

   如果不存在任何 delta specs，说明情况并 STOP。

3. **将 deltas 应用到主 specs**

   对每个 `<capability>` 的 delta：
   - 阅读 delta spec。
   - 阅读（或创建）主 spec：
     - `llmanspec/specs/<capability>/spec.md`

   按 section 手动应用：
   - `## ADDED Requirements`：添加缺失的 requirements
   - `## MODIFIED Requirements`：更新已有 requirements/scenarios
   - `## REMOVED Requirements`：删除 requirements
   - `## RENAMED Requirements`：重命名 requirements（FROM/TO 配对）

   如果需要创建新的主 spec 文件，请包含必需内容：
   - YAML frontmatter：`llman_spec_valid_scope`、`llman_spec_valid_commands`、`llman_spec_evidence`
   - `## Purpose`
   - `## Requirements`

4. **验证**

   运行：
   ```bash
   llman sdd-legacy validate --specs --strict --no-interactive
   ```

**护栏**
- 不要在 sync 中归档 change（归档使用 `/llman-sdd:archive`）
- 任何不确定之处都应先暂停并询问用户

{{ unit("skills/structured-protocol") }}
