# Design: Add `llman sdd config` interactive command

## 权衡决策

### 1. 交互库：inquire::MultiSelect vs ratatui tui_picker

| 选项 | 优点 | 缺点 |
|------|------|------|
| **inquire::MultiSelect**（选定） | 简单、扁平列表够用、符合 cli.rs:195 既有用法 | 无树形/三态（SDD 不需要） |
| ratatui tui_picker（`llman skills` 用） | 树形多选 + 搜索 | 过度设计，SDD 仅 8 个扁平选项 |

**决策**：MultiSelect。SDD optional skill 是扁平列表，无分组需求。

### 2. 注释保留：接受丢注释

`write_config`（config.rs:333）用 `serde_yaml::to_string`，丢注释。替代方案是文本定点编辑（复杂、易错）。

**决策**：接受丢注释（与现有 `write_config` 行为一致，测试已断言 `serde output should not have comments`）。写入后提示用户注释已丢失，如需注释可手工补回。

### 3. 纳入新 skill 的常量同步

3 个新 skill 要能被 `config skills` 勾选 + `init --update` 托管，必须同步 4 处：
- `OPTIONAL_SKILL_NAMES`（config.rs:14）— 校验白名单
- `OPTIONAL_SKILL_FILES`（templates.rs:27）— 模板文件名
- `templates/sdd/{en,zh-Hans}/skills/*.md`（6 个）— 模板源（MiniJinja 变量）
- schema 重生成 — 编辑器/校验器识别

漏任一处：写入报错 / init 找不到模板 / schema 漂移。

### 4. skill 描述来源

MultiSelect 每项需一行描述。两种方案：
- 从模板 frontmatter `description` 提取（动态，但需解析模板）
- 硬编码 `[(name, desc)]` 静态表（简单，8 项可控）

**决策**：硬编码静态表（`config_skills.rs` 内）。8 项可控，避免运行时解析模板的复杂度。描述与模板 frontmatter 保持一致（手动同步）。

## 复用的现有函数

| 用途 | 函数 |
|------|------|
| 读 config | `load_or_create_config`（config.rs:322） |
| 写 config | `write_config`（config.rs:333） |
| 合法值 | `OPTIONAL_SKILL_NAMES`（config.rs:14） |
| 交互判断 | `is_interactive(no_interactive)`（sdd/shared/interactive.rs） |
| 刷新 skills | `update_skills::run`（可选，写回后提示用户跑） |

## 风险

| 风险 | 缓解 |
|------|------|
| 常量与模板不同步 | tasks 里列为同一切片，一起改一起验 |
| schema 漂移 | `llman self schema generate` 重生成 + check-schemas |
| 丢注释困扰用户 | 写回后明确提示 |
