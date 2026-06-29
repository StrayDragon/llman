# Tasks: Remove the rag backend

## Phase 1 — CLI 与配置（接受 pageindex 唯一值、移除回退）

- [x] `src/sdd/command.rs`：`context`/`index rebuild` 的 `--backend` 参数校验改为仅接受 `pageindex`；传入 `rag` 等非法值时报错退出，错误提示引导配置 pageindex 所需环境变量
- [x] `src/sdd/context/mod.rs`：`Backend` enum 简化（保留 `Pageindex`，移除 `Rag`）或改为常量；`resolve_backend` 移除回退语义——`LLMAN_SDD_INDEX_BACKEND` 仅用于显式声明 pageindex，非法值报错
- [x] `src/sdd/context/mod.rs`：`context_run`/`index_rebuild` 移除 rag 分支，pageindex 成为唯一路径

## Phase 2 — 友好可操作提示（r11）

- [x] `src/sdd/context/mod.rs`：pageindex 索引缺失/过期/损坏或 chat 模型未配置时，`status.qualityNote` 与/或 stderr 输出言简意赅的可操作提示——列出缺的环境变量名（`LLMAN_SDD_INDEX_CHAT_MODEL` 等）与 rebuild 命令（`llman sdd index rebuild`），避免冗余
- [x] 单元测试：缺 `LLMAN_SDD_INDEX_CHAT_MODEL` 时的错误提示包含变量名与 rebuild 命令；传入 `--backend rag` 时报错而非 panic

## Phase 3 — 移除 rag 代码（r1/r2/r3/r6 落地）

- [x] 删除 `src/sdd/context/embed.rs`（embedding 客户端，仅 rag 用）
- [x] `src/sdd/context/mod.rs`：删除 `context_run_rag`、`index_rebuild_rag`、`embed_query`、rag 相关 `resolve_api_config`/`ApiConfig`
- [x] `src/sdd/context/index.rs`：删除 rag 专用索引结构（`ContextIndex`、`Chunk`、`ContextMetadata` 的 embedding 字段、`cosine_sim`、`z_score_normalize`、rag 的 `check_freshness` 分支）；保留 pageindex 树相关部分
- [x] 评估 `Backend` enum 是否仍有必要（pageindex 唯一后可能整个移除，简化为无 backend 概念）
- [x] `Cargo.toml`：确认 `async-openai` 仍需（pageindex chat 用）；若 embedding 不再被任何路径使用，检查 features

## Phase 4 — 文档与回归

- [x] `docs/sdd-context-index.md`：移除 rag backend 一节、移除 `LLMAN_SDD_INDEX_MODEL`/`OPENAI_*` embedding 配置说明；保留 pageindex 的 chat 配置与 r11 提示示例
- [x] `templates/sdd/<locale>/units/skills/sdd-commands.md`：更新 `--backend` 说明（仅 pageindex）
- [x] eval harness `agentdev/eval`：`rag` variant 标注为 legacy（仅可对历史 rag 索引只读验证），默认 variants 去掉 rag
- [x] 回归：`llman sdd context`（默认）与 `--backend pageindex` 行为一致；`--backend rag` 报错且提示友好

## 验证

- [x] `cargo build` 通过，无 rag 相关死代码警告
- [x] `cargo test` 通过（含 r4/r5/r11 新场景）
- [x] `cargo clippy` 通过
- [x] `llman sdd validate remove-rag-backend --strict --no-interactive` 通过
- [x] 端到端：`sdd context --backend rag` 报错并提示配置变量；`sdd context`（缺 chat model）提示友好
- [x] eval 回归：pageindex 在 xylitol 100 题上 F1 不低于基线（报告对照 `e5a96fdb`）

## 风险监控点

- 确认无其它代码/文档残留引用 `rag` backend 或 embedding 环境变量
- 确认 `async-openai` 依赖在移除 embedding 路径后仍被 chat 路径正确使用
