## 1. 核心数据结构与常量

- [x] 1.1 定义统一内部结构 `IgnoreRules { ignore, include }`（稳定排序 + 去重）
- [x] 1.2 定义 `Target` 枚举（`OpenCodeIgnore | CursorIgnore | ClaudeShared | ClaudeLocal`）
- [x] 1.3 定义默认路径解析（基于 repo root）：
  - `.ignore`
  - `.cursorignore`
  - `.claude/settings.json`
  - `.claude/settings.local.json`
- [x] 1.4 定义 warning / note 结构（用于 dry-run 预览与 `--verbose`）

## 2. 解析（Sources → IgnoreRules）

- [x] 2.1 实现 gitignore-like 解析：`.ignore` / `.cursorignore`
  - 支持注释行（`#`）与空行
  - 支持 `!pattern` 解析为 `include`
  - 其余行解析为 `ignore`

- [x] 2.2 实现 Claude Code settings 解析（JSON/JSONC）
  - 读取 `.claude/settings.json` 与 `.claude/settings.local.json`
  - 解析 `permissions.deny` 数组
  - 提取 `Read(...)` 规则为 `ignore`
  - 非 `Read(...)` 条目：保留（用于写回时不删除），并在 `--verbose` 下提示跳过

## 3. 渲染与写回（IgnoreRules → Targets）

- [x] 3.1 渲染 `.ignore` / `.cursorignore`
  - 稳定输出：先 ignore，再 include（`!` 前缀）
  - 以换行结尾，保持 deterministic diff

- [x] 3.2 写回 `.ignore` / `.cursorignore`
  - 使用原子写入（`atomic_write_with_mode` / 现有 fs 工具）
  - 保持合理权限（项目文件通常 0o644）

- [x] 3.3 Claude settings 写回：增量 union 合并到 `permissions.deny`
  - 合并策略：仅新增缺失的 `Read(./...)`（去重）
  - 不删除/不覆盖非 Read deny 条目
  - include（`!pattern`）无法写入 Claude：跳过并记录 warning

- [x] 3.4 JSONC best-effort 保留注释
  - 优先：仅局部替换 `permissions.deny` 数组区块（保留其他注释/格式）
  - fallback：无法定位时使用 pretty JSON 覆盖，并输出提示（可能丢注释）

## 4. 自动发现、git root 与 union

- [x] 4.1 实现 git root 检测：从 cwd 向上找最近 `.git`
  - 找不到时默认报错，要求 `--force`
  - `--force`：把 cwd 当 root

- [x] 4.2 自动发现 sources（存在才读）
  - `.ignore`
  - `.cursorignore`
  - `.claude/settings.json`
  - `.claude/settings.local.json`

- [x] 4.3 计算 union 并去重（ignore/include 分别 union）
  - 记录来源与 warning（用于预览与 verbose 输出）

## 5. CLI 集成（`llman tool sync-ignore`）

- [x] 5.1 在 `src/tool/command.rs` 添加 `SyncIgnore` 子命令（alias: `si`）
  - `--yes, -y`：应用写入（默认 dry-run）
  - `--interactive`：inquire 交互模式
  - `--force`：跳过 git root 检查
  - `--verbose, -v`：详细输出
  - `--target, -t <target>`：可重复，值：`opencode|cursor|claude-shared|claude-local|all`
  - `--input, -i <path>`：可重复，额外输入文件（自动识别格式）

- [x] 5.2 在 `src/cli.rs` 中处理 `ToolCommands::SyncIgnore`，调用 `tool::sync_ignore::run()`
- [x] 5.3 新增 `src/tool/sync_ignore.rs`
  - `run()`：非交互入口（自动发现 sources + union + 预览/写回）
  - `run_interactive()`：交互入口（MultiSelect targets + 删除提示 + 预览确认）
  - 默认 targets：写入/创建 `.ignore`、`.cursorignore`、`.claude/settings.json`；`.claude/settings.local.json` 仅在已存在时更新（除非显式 target）

## 6. 交互与预览（inquire）

- [x] 6.1 MultiSelect：展示 targets（标注 exists/missing）
- [x] 6.2 反选存在文件：提示“是否删除？”默认不删除
- [x] 6.3 Preview 输出（非交互与交互共用）
  - 摘要表（TARGET/ACTION/ignore/include/notes）
  - 分 target 细节：
    - `.ignore/.cursorignore`：展示将写入内容（必要时限制行数，`--verbose` 展开）
    - `.claude/settings*.json`：展示将新增的 `Read(./...)` 列表
- [x] 6.4 Confirm：确认后才执行写入/删除

## 7. x 子命令快捷方式（可选但推荐）

- [x] 7.1 `src/x/claude_code/command.rs` 添加 `sync-ignore` 子命令（alias `si`）
  - 默认 `--target claude-shared`
- [x] 7.2 `src/x/cursor/command.rs` 添加 `sync-ignore` 子命令（alias `si`）
  - 默认 `--target cursor`

## 8. i18n

- [x] 8.1 在 `locales/app.yml` 添加 `tool.sync_ignore.*`（命令描述、参数 help、交互提示、预览标题、warning/error 文本）
- [x] 8.2 所有用户可见输出使用 `t!()` 宏

## 9. 测试（仅核心逻辑；不做 inquirer 集成测试）

- [x] 9.1 单元测试：解析 `.ignore/.cursorignore`（含 `!pattern`、注释、空行）
- [x] 9.2 单元测试：解析 Claude settings（含 JSONC 注释场景）
- [x] 9.3 单元测试：union 合并去重 + 稳定输出顺序（ignore 在前，include 在后）
- [x] 9.4 单元测试：Claude `permissions.deny` patch（保留非 Read 项、只增量合并）

## 10. 文档与验证

- [x] 10.1 更新 CLI `--help` 示例（含 dry-run / `--yes` / `--target` / `--interactive`）
- [x] 10.2 运行 `just fmt`
- [x] 10.3 运行 `just lint`
- [x] 10.4 运行 `just test`
