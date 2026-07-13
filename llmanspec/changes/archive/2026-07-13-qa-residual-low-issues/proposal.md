# Proposal: QA Residual Low Issues

## Why

全量核心代码审查（`_QA_REPORT.md`）发现若干 Low 严重度问题。High/Medium 问题已在 commit `7d2e657` `cb93cb4` `e14398d` `07abaa8` `cec7073` 中修复。以下 5 个残项严重度 Low，不影响当前正确性与安全性，但代表长期风险或可维护性债务：

### 问题 A：默认 chat host 指向内部地址

`src/sdd/context/chat.rs:37-39` 硬编码默认 host 为 `http://coral:11534/v1`（内部服务名），外部用户不会暴露该地址也运行不了；若误配（没有 LLM endpoint fallback 时静默连接到此地址）可能造成意外外连或混淆。

### 问题 B：配置/数据文件读取无大小上限

多处 `read_to_string` + YAML/TOML 解析（`src/config_schema.rs`, `src/sdd/context/index.rs` 等）无文件大小上限。恶意超大配置可造成内存尖峰（DoS）。虽然本地攻击面低，但 CI/共享 runner 上可能被利用。

### 问题 C：`SQLITE_OPEN_NO_MUTEX` 线程安全风险

`src/x/cursor/database.rs` 用 `SQLITE_OPEN_NO_MUTEX` 标志打开 SQLite 连接。若未来连接跨线程使用则不安全。当前单线程使用风险低，但缺少防御性措施。

### 问题 D：`persist` 写入 symlink 目标

`src/fs_utils.rs:7-28` 的 `persist` 若目标是 symlink 会写入链接目标而非替换链接。并发场景下 last-wins，可能造成非预期覆盖。

### 问题 E：代码质量债务

- 多套 `Config` 命名并存（`crate::config` / `tool::config` / `x::claude_code::config`），阅读成本高
- `managed_block` 与 completion marker 逻辑有重复
- 若干模块体量过大（~1k–1.7k LOC）：`skills/cli/command.rs`、`sdd/spec/validation.rs`、`skills/targets/sync.rs`、`x/codex/agents.rs`、`tool/sync_ignore.rs`、`usage_stats/tui.rs`

## What Changes

### 1. 默认 chat host 安全默认值

- 将默认 host 从 `http://coral:11534/v1` 改为空字符串或 `http://localhost:11534/v1`（或更安全的不可用值）
- 当 host 解析为不安全/内部地址时输出警告
- 在配置文档中明确标注默认值含义

### 2. 文件读取大小上限

- 为 `read_to_string` + 结构化解析（YAML、TOML）统一添加配置大小上限（建议 10 MiB）
- 超过上限时返回清晰错误，而非 OOM
- 新增 `fs_utils::read_with_max_size` 或类似辅助函数

### 3. SQLite 打开标志加固

- 移除 `SQLITE_OPEN_NO_MUTEX` 或添加注释说明为何单线程安全
- 如果将来要跨线程使用，考虑用 `SQLITE_OPEN_FULL_MUTEX` 或记录约束

### 4. `persist` symlink 安全写入

- 在 `persist` / `atomic_write_*` 中检测目标是否为 symlink，若是则写入临时文件后 `rename`（或明确拒绝）
- 统一 `fs_utils.rs` 的写入语义

### 5. 代码质量（渐进）

- 统一 Config 类型命名：`tool::config` → `tool::ToolConfig`、`x::claude_code::config::Config` → `x::claude_code::config::ClaudeCodeConfig`
- 提取 `managed_block` 与 completion marker 公共逻辑
- 大模块拆分作为长期路线图，本 change 仅创建 issue 跟踪

## Capabilities

- `sdd-context`: chat 默认 host 加固（r12 新增）
- `config-paths`: 文件读取大小上限 + `persist` symlink 安全（r5–r6 新增）
- `cursor-export`: SQLite 线程安全约束（r8 新增）
- 代码质量项直接作为任务，不引入新 spec

## 修复优先级（本 change 内）

1. 默认 host 加固（行数少、影响明确）
2. `persist` symlink 安全（fs_utils.rs，单一职责，容易覆盖测试）
3. 文件读取大小上限（跨模块，需新增辅助函数）
4. SQLite 标志加固（单文件改动）
5. 代码质量（Config 命名重命名 → 模块拆分 → 去重，渐进）

## 待定问题

1. 默认 host 改为空字符串还是 `localhost`？——建议空字符串，引导用户显式配置
2. 文件大小上限设为多少？——建议 10 MiB，与常见 CI artifact 对齐
3. Config 重命名是否会造成破坏性变更？——不会，因为 `tool::config` 和 `x::claude_code::config` 都是内部 use，不暴露在 CLI API 中
