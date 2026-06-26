# Proposal: 将 Embedding 从 Python 脚本迁移到原生 Rust HTTP

## Why（为什么）

0.0.50 引入的 `llman sdd context / index` 功能通过 `scripts/embed_chunks.py` Python 脚本调用 embedding API，存在以下问题：

1. **外部依赖**：用户需要安装 Python 3 + `requests` 包才能使用 index rebuild。脚本路径也容易因工作目录不同而找不到（`Script not found: scripts/embed_chunks.py`）。
2. **可靠性**：Python 进程 spawn/pipe 通信相比原生 HTTP 调用更容易出错，且缺少 Rust 编译期的类型保证。
3. **可维护性**：embedding 相关的逻辑分散在 Rust 与 Python 两个语言之间，不利于统一维护与测试。
4. **环境变量前缀不一致**：当前使用 `LLMAN_EMBED_*` 前缀，但该功能属于 SDD index 子系统，应使用 `LLMAN_SDD_INDEX_*` 前缀以便与其他 llman 配置区分。

## What Changes（变更内容）

1. **移除 Python 脚本依赖**：将 `src/sdd/context/mod.rs` 中的 `embed_query()` 和 `index_rebuild()` 调用 Python 脚本的逻辑替换为原生 Rust HTTP 客户端调用。
2. **新增 `LLMAN_SDD_INDEX_*` 环境变量**：提供 `LLMAN_SDD_INDEX_OPENAI_API_HOST`、`LLMAN_SDD_INDEX_OPENAI_API_KEY`、`LLMAN_SDD_INDEX_MODEL` 三个专用环境变量，作为全局配置源（优先级：CLI 参数 > 环境变量 > 硬编码默认值）。
3. **添加 HTTP 客户端依赖**：在 `Cargo.toml` 中引入轻量级 HTTP 客户端（如 `ureq` 或 `reqwest` 的 blocking 模式），用于发送 embedding API 请求。
4. **实现内容感知的 Index Rebuild 进度**：保留 Python 脚本中原有的 batch 处理（batch_size=8）与重试逻辑，在 Rust 端原生实现。

## Capabilities（影响能力）

- `sdd-context`：`src/sdd/context/mod.rs`（embedding 调用逻辑）、`src/sdd/context/index.rs`（索引结构保持不变）、`Cargo.toml`（新增依赖）

## Impact（影响评估）

- 向后兼容：`LLMAN_EMBED_*` 环境变量不再读取，用户需迁移到新环境变量或使用 CLI 参数。
- 移除文件：`scripts/embed_chunks.py` 不再被 llman 调用，可作为死代码清理或在后续变更中删除。
- 测试：embedding API 调用需要通过 integration 测试或 mock 来覆盖（当前缺少现成的 mock 基础设施——本次变更暂不添加 mock 框架，仅确保编译通过且与现有行为等价）。
- 风险等级：low（替换行为等价逻辑，不改变 embedding 输入/输出格式）。

## Ethics

- `ethics.risk_level`: low
- `ethics.prohibited_actions`: 不得修改 embedding API 的请求/响应数据格式（保持与 Python 脚本一致的 OpenAI-compatible API 协议）。
- `ethics.required_evidence`: 实现后需通过 `just build`（debug 构建成功）和 `llman sdd index rebuild`（指向一个真实的 API endpoint 或 mock）验证。
- `ethics.refusal_contract`: 若现有 embedding index 格式与 Python 脚本输出存在差异，不应修改 index 格式，仅替换调用方式。
- `ethics.escalation_policy`: 若需要更改 embedding API 协议/格式（如切换到非 OpenAI-compatible），需升级为 separate change。
