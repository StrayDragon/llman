## Context

- 现状：`llman x codex account edit` 已提供“用 `$VISUAL/$EDITOR` 打开配置文件”的入口，并支持 `$EDITOR` 包含参数（例如 `code --wait`）。
- `llman x claude-code account` 目前提供 `list/import/use`，但缺少 `edit`，导致用户只能手动定位配置文件路径再打开编辑器，体验不一致。
- Claude Code 配置文件由 `src/x/claude_code/config.rs::Config::config_file_path()` 解析：优先 `LLMAN_CONFIG_DIR`，否则使用 ProjectDirs（Linux 通常为 `~/.config/llman/claude-code.toml`）。
- Claude Code 的 TOML 解析对“空文件/缺失 groups 字段”更敏感：若文件存在但内容不满足结构，`Config::load()` 会报错。

## Goals / Non-Goals

**Goals:**
- 新增 `llman x claude-code account edit`，用 `$VISUAL/$EDITOR` 打开 `claude-code.toml`。
- 与 Codex 的 edit 行为对齐：支持 editor 命令带参数，并将配置文件路径作为最后一个参数追加。
- 当配置文件不存在时创建一个“最小可解析”的默认模板，避免用户第一次打开后留空导致后续命令解析失败。
- 用自动化测试覆盖：editor 参数追加、文件创建、非零退出等分支（测试中使用 `LLMAN_CONFIG_DIR`，不触碰真实用户配置）。

**Non-Goals:**
- 不改变 `llman x claude-code account` 的默认行为（保持现有 `None => list`，避免破坏现有脚本/习惯）。
- 不引入新的配置迁移机制或重构 Claude Code 的配置格式。
- 不额外扩展 Windows 交互编辑体验（保持 best-effort，与现有平台策略一致）。

## Decisions

1) **共享 editor 选择与解析逻辑**
- 方案：将 Codex 中的 `select_editor_from_env` / `parse_editor_command` 抽到共享模块（例如 `src/editor.rs` 或 `src/tool/editor.rs`），Claude Code 与 Codex 共用。
- 理由：减少重复与漂移，保证两套子命令行为一致（优先 `$VISUAL`，其次 `$EDITOR`，都为空则回退到 `vi`；解析失败给出可读错误）。
- 替代：在 `claude_code/command.rs` 复制一份实现；缺点是未来维护成本更高。

2) **为 claude-code.toml 提供默认模板**
- 方案：新增 `templates/claude-code/default.toml`（或在代码内置字符串），在 `account edit` 发现文件不存在时写入。
- 模板要求：包含最小结构（例如至少有 `[groups]`），并可用注释提供示例配置，确保 `Config::load()` 不因“空文件”失败。
- 兼容性：与 `LLMAN_CONFIG_DIR` 解析一致；不改变现有 import 流程。

3) **CLI 结构与输出**
- 在 `src/x/claude_code/command.rs` 为 `AccountAction` 增加 `Edit`，并在 `execute_account_action` 中路由到 `handle_account_edit`。
- 输出与错误使用 `t!` key（新增 `claude_code.account.*` / `claude_code.error.*`），并对 editor 非零退出返回明确错误信息。
- 利用 `x` 命令的别名：`llman x cc account edit` 自动生效（无需额外实现）。

4) **测试策略**
- 使用一个临时可执行脚本作为“假 editor”，通过 `$EDITOR` 注入。
- 脚本将收到的参数写入临时文件，测试断言最后一个参数是 `claude-code.toml` 路径。
- 覆盖场景：首次创建模板、editor 带参数（例如 `fake-editor --wait`）、editor 返回非零退出码。

## Risks / Trade-offs

- [风险] 在非交互环境中 fallback 到 `vi` 可能导致挂起 → [缓解] 与 Codex 保持一致；测试通过显式设置 `$EDITOR` 避免该路径。后续如需更严格，可另起变更引入“非交互环境拒绝打开交互编辑器”的行为。
- [风险] 默认模板内容与未来配置 schema 演进不一致 → [缓解] 保持模板最小化（仅保证可解析），示例内容用注释呈现，避免误导或强绑定字段。
- [风险] 配置文件可能包含敏感信息，默认权限不安全 → [缓解] 在 Unix 上创建模板后设置 `0600`（与现有 `Config::save_to_path` 行为对齐）。
