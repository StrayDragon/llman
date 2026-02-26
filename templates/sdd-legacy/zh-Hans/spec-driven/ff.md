<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/ff.md (copied 2026-02-09) -->

快速推进规划工件创建——在 llman SDD 中生成开始实施所需的一切。

**输入**：`/llman-sdd:ff` 后面的参数是变更 id（kebab-case），或用户想要构建内容的描述。

**步骤**

1. **如果没有明确输入，先问清楚要做什么**

   询问：
   > "你想做什么变更？描述你想要构建或修复的内容。"

   根据描述派生 kebab-case 的 id（例如："add user authentication" → `add-user-auth`）。

2. **确保项目已初始化**

   检查是否存在 `llmanspec/`。若不存在，提示用户先运行 `llman sdd init`，然后 STOP。

3. **创建变更目录**

   若不存在则创建：
   - `llmanspec/changes/<id>/`
   - `llmanspec/changes/<id>/specs/`

   如果变更已存在，询问用户是否：
   - 继续补齐缺失工件（推荐），或
   - 改用其他 id。

4. **创建规划工件（spec-driven）**

   按顺序创建：

   a) `proposal.md`
   - 写清 Why / What Changes / Capabilities / Impact
   - 若范围不清晰，先问 1–2 个关键澄清问题再写

   b) `specs/<capability>/spec.md`（按 capability）
   - 对 proposal 中列出的每个 capability，在下列位置创建 delta spec：
     `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - 使用 `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`，并为每条 requirement 至少写一个 `#### Scenario:`

   c) `design.md`（可选）
   - 若变更跨多个系统、风险高或需要权衡：创建 `design.md`
   - 否则可跳过（或仅在用户要求时写简短 stub）

   d) `tasks.md`
   - 将实施拆成可勾选的小任务
   - 包含验证命令（例如 `just check`、`llman sdd validate <id> --strict --no-interactive`）

5. **建议校验并交接给实施**

   建议运行：
   - `llman sdd validate <id> --strict --no-interactive`

   然后提示：
   - "准备开始实施。运行 `/llman-sdd:apply <id>`。"

**护栏**
- 不要实现应用代码
- 工件保持最小化、与用户请求紧密一致
- 不要擅自扩大范围；不确定就先问

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
