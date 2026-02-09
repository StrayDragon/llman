<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/zh-Hans/opsx/new.md (copied 2026-02-09) -->

使用 llman SDD 的 OPSX 动作式工作流开始新变更。

**输入**：`/opsx:new` 之后的参数是变更 id（kebab-case），或用户想要构建内容的描述。

**步骤**

1. **如果没有明确输入，先问清楚要做什么**

   询问：
   > "你想做什么变更？描述你想要构建或修复的内容。"

   根据描述派生 kebab-case 的 id（例如："add user authentication" → `add-user-auth`）。

   **重要**：在你理解用户要做什么之前，不要继续。

2. **确保项目已初始化**

   检查仓库根目录是否存在 `llmanspec/`。
   - 若不存在：提示用户先运行 `llman sdd init`，然后 STOP。

3. **创建变更目录（先不创建工件）**

   创建：
   - `llmanspec/changes/<id>/`
   - `llmanspec/changes/<id>/specs/`

   如果该变更已存在，建议改用 `/opsx:continue <id>`。

4. **STOP 并等待用户下一步指示**

**输出**

完成步骤后，总结：
- 变更 id 与位置（`llmanspec/changes/<id>/`）
- 当前状态（尚未创建任何工件）
- 提示："准备好创建第一个工件了吗？运行 `/opsx:continue <id>`（或告诉我下一步要做什么）。"

**护栏**
- 不要实现应用代码
- 不要创建任何变更工件（proposal/specs/design/tasks）——交给 `/opsx:continue` 或 `/opsx:ff`
- 如果 id 无效（非 kebab-case），请询问有效 id
