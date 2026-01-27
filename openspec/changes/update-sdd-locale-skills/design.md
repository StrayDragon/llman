## Context

- 现有 sdd 模板固定为英文且通过 `include_str!` 编译期加载，无法根据用户语言切换。
- `llmanspec/AGENTS.md` 仅包含简短提示块，AI 无法获取完整 llman sdd 方法论。
- 未提供 sdd 专用 skills 的生成/更新入口，用户需要手工维护。
- CLI i18n 仅英文稳定，locale 需求集中在模板与 skills 输出。

## Goals / Non-Goals

**Goals:**
- 提供项目级 `llmanspec/config.yaml` 用于 locale 与 skills 路径配置
- 基于 locale 加载 `llmanspec/AGENTS.md`、模板与 skills 内容
- `llman sdd init/update` 自动维护 root `AGENTS.md` stub
- 新增 `llman sdd update-skills` 用于生成/刷新 Claude Code 与 Codex skills
- 在 validate 输出中补充可行动的修复提示

**Non-Goals:**
- 不改动 CLI 的语言策略（保持英文输出）
- 不引入 slash commands 或其他外部工具注入
- 不改动现有 `llman skills` 子命令逻辑

## Decisions

1. **新增项目级配置 `llmanspec/config.yaml`**
   - 结构（示例）：
     ```yaml
     version: 1
     locale: en
     skills:
       claude_path: .claude/skills
       codex_path: .codex/skills
     ```
   - `llman sdd init --lang <locale>` 写入 `locale`；默认 `en`。
   - `llman sdd update` 与 `llman sdd update-skills` 读取该配置；若缺失则回退默认并可写入默认配置。

2. **Locale 解析与回退链**
   - 支持 `en` 与 `zh-Hans`，并提供回退：`zh-Hans` → `zh` → `en`。
   - locale 仅用于模板与 skills 输出，不影响 CLI 文本；`LLMAN_LANG` 不参与 sdd locale 解析。

3. **模板目录结构**
   - 使用 `templates/sdd/<locale>/` 组织模板：
     - `agents.md`（完整方法论）
     - `agents-root-stub.md`（root `AGENTS.md` 托管块）
     - `project.md`
     - `spec-driven/*.md`
     - `skills/*.md`

4. **root `AGENTS.md` Stub 管理**
   - `llman sdd init/update` 创建或刷新 root `AGENTS.md` 中的受管块，指向 `llmanspec/AGENTS.md`。
   - 受管块外内容保留不变。

5. **Skills 生成策略**
   - 新增 `llman sdd update-skills`：交互模式选择 Claude Code/Codex，默认路径来自配置并允许输入自定义路径。
   - 非交互模式支持 `--all` 与 `--tool claude,codex`（或多次传入），可选 `--path`；若缺少 tool 参数则报错。
   - 仅生成 skills（`<tool>/skills/<skill>/SKILL.md`），不生成 slash commands。
   - 生成时可直接覆盖 `SKILL.md` 以保证托管一致性。

6. **Skills 内容与校验提示**
   - 技能模板包含 `llman sdd` 命令使用说明，并嵌入 validate 的修复提示与最小示例。

7. **Region 引用与模板复用**
   - 提供通用 region 解析器，从源文件中读取 `region` 块并注入到模板。
   - 模板占位符格式：`{{region: <path>#<name>}}`（path 为 repo 根目录相对路径）。
   - region 注释语法按文件类型决定：
     - Markdown/HTML：`<!-- region: name -->` / `<!-- endregion -->`
     - YAML/TOML/INI/Shell：`# region: name` / `# endregion`
     - Rust/JS/TS：`// region: name` / `// endregion`
   - 若 region 未找到或重复，默认报错并中止生成（避免静默缺失）。

## Risks / Trade-offs

- 新增 `llmanspec/config.yaml` 引入额外文件维护成本。
- skills 文件为托管内容，覆盖更新可能影响用户的本地自定义。
- locale 仅用于模板/skills，CLI 仍为英文，可能造成预期差异；需在文档中明确。
