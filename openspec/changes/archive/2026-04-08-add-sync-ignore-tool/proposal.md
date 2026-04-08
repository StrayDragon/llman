## Why

开发者经常同时使用多个 Agent 工具（OpenCode / Cursor / Claude Code）。这些工具都会自动遵守 `.gitignore`，但它们各自还提供了额外的“搜索/可见文件范围”配置入口：

- OpenCode：项目根 `.ignore`（底层工具使用 ripgrep；可用 `!pattern` 显式包含）
- Cursor：项目根 `.cursorignore`（gitignore 风格）
- Claude Code：项目内 `.claude/settings.json` / `.claude/settings.local.json` 的 `permissions.deny`（`Read(...)` 规则）

在缺乏统一规范时，团队需要手动维护多份规则，容易漏掉敏感文件或导致不同工具行为不一致。

## What Changes

为 `llman tool` 添加 `sync-ignore` 工具（别名 `si`），实现：

1. 读取任意一种输入格式，并统一解析为内部结构 `{ ignore, include }`
2. 自动识别项目内所有已存在 ignore 配置并取 union（并集），得到一份“统一规则集”
3. 将统一规则集同步/转换为其它工具格式，并提供预览（默认 dry-run）与交互选择

### New CLI

- `llman tool sync-ignore`（alias: `si`）
  - 默认：**dry-run**（只预览不写入）
  - 默认：要求在 git repo 内运行（找不到 `.git` 需显式 `--force`）
  - 自动发现 sources（存在才读）：`.ignore`、`.cursorignore`、`.claude/settings.json`、`.claude/settings.local.json`
  - 默认 targets（非交互）：
    - 写入/创建：`.ignore`、`.cursorignore`、`.claude/settings.json`
    - `.claude/settings.local.json`：仅在已存在时更新（避免意外创建本机配置）
  - `--yes, -y`：应用写入（会自动创建缺失的 target 文件/目录）
  - `--target, -t <target>`：选择输出目标（可重复）
    - 值：`opencode|cursor|claude-shared|claude-local|all`
  - `--input, -i <path>`：额外指定输入文件（可重复；支持自动格式识别）
  - `--interactive`：inquire 交互模式（MultiSelect 选择输出文件；反选存在文件会询问是否删除，默认不删；展示预览；确认后写入）
  - `--force`：跳过 git root 检查，把当前目录作为 root
  - `--verbose, -v`：显示每条规则的转换、跳过与警告

### Format Mapping

- `.ignore` / `.cursorignore`：
  - `pattern` → `ignore(pattern)`
  - `!pattern` → `include(pattern)`
- Claude Code settings：
  - 仅处理 `permissions.deny` 中的 `Read(...)`：
    - `Read(./secrets/**)` → `ignore("secrets/**")`
  - `include(!pattern)` 无法映射到 Claude Code（deny-only）→ 跳过并警告

### Claude Code JSONC preservation (best-effort)

写入 `.claude/settings*.json` 时，优先“局部替换 `permissions.deny` 数组区块”以保留 JSONC 注释与文件风格；定位失败时 fallback 为 pretty JSON 覆盖并提示风险。

## Capabilities

### New Capabilities
- `unified-ignore-sync`：在 OpenCode `.ignore`、Cursor `.cursorignore` 与 Claude Code `.claude/settings*.json` 之间解析/转换并执行 union 同步。

### Modified Capabilities
- 无（新增工具，不改变现有命令）

## Impact

**代码变更（预期）：**
- `src/tool/command.rs`：为 `ToolCommands` 枚举添加 `SyncIgnore` 子命令
- `src/tool/sync_ignore.rs`：新增核心实现（解析、union、渲染、预览、写入、JSONC patch）
- `src/tool/mod.rs`：导出新模块
- `src/x/claude_code/command.rs`、`src/x/cursor/command.rs`：可选快捷转发（若保留）
- `locales/app.yml`：添加 i18n 字符串（帮助文本与交互提示）
- `tests/**`：仅核心逻辑测试（不覆盖 inquirer 交互）

**新依赖：** 无（复用现有 `ignore`, `inquire`, `serde_json`, `llm_json`, `anyhow`）

**用户可见变更：**
- 新 CLI 命令：`llman tool sync-ignore` / `llman tool si`
- 可能创建/更新文件：`.ignore`, `.cursorignore`, `.claude/settings.json`, `.claude/settings.local.json`
