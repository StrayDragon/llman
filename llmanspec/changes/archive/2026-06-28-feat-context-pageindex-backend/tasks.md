# Tasks: PageIndex Backend for sdd context/index

## Phase 1 — 基建（依赖、参数、配置、async、路径隔离）

- [x] `Cargo.toml`：移除 `reqwest`，新增 `async-openai`；确认 `tokio` features 覆盖 rt-multi-thread
- [x] `src/sdd/command.rs`：`context` 与 `index rebuild` 子命令新增 `--backend <rag|pageindex>` 参数（clap derive，默认 pageindex）
- [x] `src/sdd/context/mod.rs`：新增 `Backend` enum 与 `resolve_backend(cli, env)` 解析（优先级 CLI > `LLMAN_SDD_INDEX_BACKEND` > 默认 pageindex）
- [x] `src/sdd/context/index.rs`：索引路径改为 backend 隔离（`.context/rag/` 与 `.context/pageindex/`）；`check_freshness` 接受 backend 参数
- [x] `src/sdd/context/index.rs`：旧布局兼容（检测 `.context/metadata.toml` 直接位于 `.context/` 时迁移到 `.context/rag/`）
- [x] `src/sdd/command.rs`：context 子命令入口用 `tokio::runtime::Runtime::new()?.block_on()` 包裹 async（不扩散 async 到其他子命令）
- [x] `src/sdd/context/mod.rs`：`context_run` 改为 `async fn`，按 backend 分发（此阶段两分支都返回「索引缺失」错误）

> 实施注记：`embed.rs` 的 reqwest→async-openai 迁移（任务上原列在 Phase 3）随 Phase 1 一并完成——因为 Phase 1 从 Cargo.toml 移除 reqwest 后，`embed_texts` 必须同步迁移才能编译。已在 design.md「实施决策记录」中说明。

## Phase 2 — pageindex 建树（tree.rs，无 LLM）

- [x] 新增 `src/sdd/context/tree.rs`：定义 `DocNode`/`ReqNode`/`TreeIndex` 结构
- [x] `tree.rs`：实现 `build_tree_from_specs(specs: &[Spec]) -> TreeIndex`（spec IR → 树映射）
- [x] `tree.rs`：实现 `TreeIndex::save/load(path)` 序列化到 `tree.json`
- [x] `tree.rs`：单元测试（mock Spec IR → 验证树结构、req_id 保留、spec_hash 记录）
- [x] `src/sdd/context/index.rs`：`index_rebuild` 的 pageindex 分支调用 `build_tree_from_specs` + `save`（无 LLM 请求）
- [x] 验证：`llman sdd index rebuild --backend pageindex` 在 llman 自己的 llmanspec 上生成 `.context/pageindex/tree.json`

> 实施注记：`build_docs` 消费 spec IR `MainSpecDoc`（经 `BACKEND.parse_main_spec`）而非 parser 的 `Spec`，因为后者在转换时丢掉了 `req_id`/`title`（见 design.md 决策 1）。`index_rebuild_pageindex` 位于 `mod.rs`。

## Phase 3 — pageindex 检索（chat.rs + retrieve.rs，agentic）

- [x] 新增 `src/sdd/context/chat.rs`：`ChatConfig` 结构 + `resolve_chat_config(embed_cfg)` 回退逻辑（`LLMAN_SDD_INDEX_CHAT_*` → `LLMAN_SDD_INDEX_OPENAI_*`）
- [x] `chat.rs`：封装 `async-openai::Client` 的 chat + tool-calling 请求（system/user/tools → response）
- [x] `src/sdd/context/embed.rs`：reqwest → async-openai 的 `embeddings()`；保留 `embed_texts` 接口语义（批处理 + 重试交由库处理）
- [x] 新增 `src/sdd/context/retrieve.rs`：三工具 schema（`list_specs`/`get_document_structure`/`get_spec_content`）
- [x] `retrieve.rs`：`dispatch_tool(call, tree) -> String` 本地执行工具（读 TreeIndex，无网络）
- [x] `retrieve.rs`：`retrieve_via_pageindex(tree, task, paths, chat_cfg) -> RetrievalOutput` agentic loop（含 `MAX_TOOL_ROUNDS=8` 上限）
- [x] `retrieve.rs`：`parse_final_answer(content) -> RetrievalOutput` 解析 LLM 最终 JSON 到 `{direct, related}`
- [x] `retrieve.rs`：单元测试（mock chat client 预设 tool_calls 序列 → 验证 loop 控制流、轮次上限、解析）
- [x] `src/sdd/context/mod.rs`：`context_run` 的 pageindex 分支调用 `retrieve_via_pageindex`，`quality` 标记 `agentic`
- [x] 端到端验证：`llman sdd context --task "add a new validation rule" --backend pageindex` 返回 `sdd-workflow` 在 direct

> 实施注记：
> - 为支持无网络环境下的单元测试，`retrieve.rs` 的 agentic loop 泛型于 `I: ChatInvoker`（edition 2024 原生 `async fn in trait`，无 `async-trait` 依赖）。真实 async-openai 调用封装在 `chat.rs` 的 `OpenAiInvoker`（`impl ChatInvoker`）。
> - `ChatConfig::from_env` 未设置 chat model 时报错（区别于 embedding，chat 模型无默认值），对齐 r7「CHAT_* 回退 OPENAI_*」。
> - 自举验证（用 `deepseek-chat` 经 NEWAPI endpoint）三场景全部命中正确（见任务末验证节）。
> - `reason` 字段由 LLM 推理给出（非 `semantic match`），输出 JSON 结构与 rag 对齐，对齐 r5。

## Phase 4 — 默认切换、回归、文档

- [x] 确认 `--backend` 默认值为 pageindex（CLI + env 默认）
- [x] 回归测试：`--backend rag` 行为与改动前完全一致（向量检索、quality=semantic）
- [x] 更新 `templates/sdd/<locale>/skills/sdd-commands.md`：补充 `--backend rag|pageindex` 说明
- [x] 更新 skill 模板中 context 命令示例（onboard/explore/propose/apply）说明默认 agentic 检索
- [x] README / 文档：新增 `LLMAN_SDD_INDEX_CHAT_MODEL` 等配置项说明（见 `docs/sdd-context-index.md`）

## 验证

- [x] `cargo build` 通过，无 reqwest 直接依赖
- [x] `cargo test` 通过（含新增 tree/retrieve 单测）
- [x] `cargo clippy` 通过
- [x] `llman sdd validate --strict --no-interactive` 通过（本 change 的 delta spec）
- [x] llman 自举验证：用 pageindex backend 检索 llman 自己的 specs，关键场景命中正确：
  - task="修改 validate 退出码" → direct 含 sdd-workflow / errors-exit
  - task="改 prompt 模板" → direct 含 prompts-management / sdd-template-units-and-jinja
  - task="typo fix" → direct 为空或仅 cli-experience（quick path 信号）

## 风险监控点

- [x] agentic 检索延迟（记录 `toolCalls` 数到 summary，便于观测）
- [x] chat 模型 tool-calling 质量（若弱模型漏调工具，记录 qualityNote 提示换模型）
- [x] 旧索引迁移（确保现有用户 `--backend rag` 不丢索引）

> 实施注记（Phase 4 + 验证 + 风险）：
> - 默认 backend 已确认为 pageindex（`resolve_backend`：CLI > `LLMAN_SDD_INDEX_BACKEND` > 默认 pageindex）；rag 回归实测两 baseline 任务均返回 `quality=semantic` 且命中 specs 与改动前一致（coral embedding 服务恢复后实跑）。
> - 自举验证（用 `deepseek-chat` 经 NEWAPI endpoint）三场景多次重复稳定命中：S1 validate-exit 6/6、S2 prompt-template 4/4、S3 typo quick-path 4/4。
> - 风险点「agentic 检索延迟」：`summary.toolCalls` 已记录（实测 9–17 次/查询）；「tool-calling 质量」：轮次超限强制无工具收尾时写 `qualityNote`，并提供 `LLMAN_SDD_INDEX_DEBUG=1` 轨迹；「旧索引迁移」：`resolve_backend_dir(Rag)` 读时回退旧平铺布局，且现有用户运行 `--backend rag` 自然写入新 `.context/rag/`。
> - 实施中发现并修正 design.md 偏差：`MAX_TOOL_ROUNDS` 由 8 提升至 12，并增加超限强制无工具收尾（见 design.md 决策 5）。
