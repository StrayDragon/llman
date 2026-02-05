## Why
`llman prompts` 当前实现存在若干边界问题，会导致：
- `list` 能看到模板但 `gen` 读不到（报 “rule not found”），用户难以理解和排查。
- Claude Code 注入在读取既有 `CLAUDE.md` 失败时可能“静默当作空文件”，存在覆盖风险。
- `rm` 只能交互确认，导致 CI/脚本无法稳定执行模板删除。

这些问题会降低命令的可预期性与安全性，且与 `prompts-management` 规范期望的“一致冲突策略/非交互拒绝覆盖”等原则不完全一致。

### Current Behavior（基于现有代码）
- 模板列举：`Config::list_rules` 仅按文件 stem 列出目录下所有文件，不按扩展名过滤（`src/config.rs`）。
- 模板读取：`PromptCommand::get_template_content` 通过 `rule_file_path(app, template)` 以“固定扩展名”拼接路径读取（`src/prompt.rs`），与列举行为不一致。
- Claude 注入：读取已存在 `CLAUDE.md` 使用 `unwrap_or_default()`（`src/prompt.rs`），读取失败会被吞掉。
- 项目校验：Cursor 的项目目录校验基于当前工作目录是否存在 `.git`（`src/prompt.rs`），在 repo 子目录中运行可能误判。

## What Changes
- 让模板“列举”与“读取”在每个 app 维度完全一致：只展示/只读取支持的扩展名文件，避免选中后读不到。
- 为 `llman prompts rm` 增加显式 `--yes`（或等价语义）以支持非交互删除；非交互且未显式确认时拒绝删除并返回错误。
- Claude Code memory 注入：当目标文件存在但不可读（I/O、非 UTF-8 等）时必须失败并停止写入，避免静默覆盖。
- 项目 scope root 解析：在 repo 子目录运行时也能正确定位 `<repo_root>`（向上查找 git root），并用于 project-scope 输出路径。

### Non-Goals（边界）
- 不更改模板存储目录结构（仍为 `LLMAN_CONFIG_DIR/prompt/<app>/`）。
- 不改变现有 Cursor/Codex/Claude 的输出格式与托管块标记策略（仅修复错误路径/安全行为）。
- 不新增新的 app 类型。

## Impact
- Affected specs: `specs/prompts-management/spec.md`
- Affected code:
  - `src/config.rs`（模板列举/定位规则）
  - `src/prompt.rs`、`src/cli.rs`（参数、project root 解析、注入安全）
- User-visible:
  - `prompts list/gen` 行为更一致，减少 “list 可见但 gen 读不到”。
  - `prompts rm` 支持脚本化删除（必须显式 `--yes`）。
  - Claude 注入在读取失败时改为显式报错（避免静默覆盖）。
