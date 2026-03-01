<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/new.md (copied 2026-02-09) -->

在 llman SDD 中开始一个新变更（只创建目录；暂不创建工件）。

**输入**：`/llman-sdd:new` 之后的参数要么是：
- 变更 id（kebab-case），要么是
- 一段简短描述（你需要派生 id 并让用户确认）。

**步骤**

1. **确定 change id**

   若提供了 id，直接使用。否则：
   - 先询问用户想构建/修复什么。
   - 提议一个 kebab-case 的 id（例如："add user authentication" → `add-user-auth`）。
   - 在创建任何目录前，先让用户确认该 id。

   若 id 无效或不明确，**STOP** 并继续澄清。

2. **确保项目已初始化**

   检查仓库根目录是否存在 `llmanspec/`。
   - 若不存在：提示用户先运行 `llman sdd-legacy init`，然后 STOP。

3. **创建变更目录（不创建工件）**

   创建：
   - `llmanspec/changes/<id>/`
   - `llmanspec/changes/<id>/specs/`

   如果该变更已存在，建议改用 `/llman-sdd:continue <id>`。

4. **STOP 并等待用户下一步指示**

**输出**

完成步骤后，总结：
- 变更 id 与位置（`llmanspec/changes/<id>/`）
- 当前状态（尚未创建任何工件）
- 提示："准备好创建第一个工件了吗？运行 `/llman-sdd:continue <id>`。"
- 备选："想一次性把工件都创建好？运行 `/llman-sdd:ff <id>`。"

**护栏**
- 不要实现应用代码
- 不要创建任何变更工件（proposal/specs/design/tasks）——交给 `/llman-sdd:continue` 或 `/llman-sdd:ff`
- 不要猜 id；若 id 无效（非 kebab-case），请询问有效 id

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
