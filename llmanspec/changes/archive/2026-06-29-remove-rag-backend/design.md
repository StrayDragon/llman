# Design: Remove the rag backend

> 技术设计。与 `proposal.md` 的「为什么」互补，本文聚焦「怎么做」与权衡。

## 决策概览

本次变更核心是**做减法**：移除 rag backend 及其整条代码路径，让 pageindex 成为唯一
检索后端。对外 CLI 契约收紧（`--backend` 仅接受 pageindex），对内删除 embedding 相关
模块。

## 关键决策

### 决策 1：`--backend` 保留但收紧为单值，而非整体移除参数

考虑过两个方案：
- **A. 完全删除 `--backend` 参数**：最干净，但破坏所有已使用 `--backend pageindex` 的
  脚本/文档，且未来若新增其它检索策略需重新加参数。
- **B. 保留 `--backend`，仅接受 `pageindex`**（采用）：向后兼容现有显式写法，非法值
  （含 `rag`）报错并提示。`LLMAN_SDD_INDEX_BACKEND` 同理保留但仅声明 pageindex。

采用 B：破坏性更小，且报错本身是引导用户认知「只有 pageindex」的触点。

### 决策 2：rag 代码完全物理删除，不保留死代码

`embed.rs`、`context_run_rag`、`index_rebuild_rag`、rag 索引结构（`Chunk`、
`ContextMetadata` 的 embedding 字段、`cosine_sim`、`z_score_normalize`）、rag 的
`check_freshness` 分支——全部删除。理由：
- 保留死代码 = 持续维护成本 + 误导未来贡献者「这还在用」。
- git 历史可追溯，需要时从历史恢复。
- eval harness 的 `rag` variant 是**独立**的 Bun 代码，不依赖 Rust rag 实现，故 eval
  仍可跑 rag 作为历史对照（见决策 4）。

需注意 `index.rs` 中部分类型被 pageindex 路径间接复用（如 `compute_spec_hash`、
`backend_subdir`、`IndexFreshness`、`check_freshness` 的骨架），这些**保留**，只删
rag 专属部分。

### 决策 3：`async-openai` 依赖保留

r6 原文要求「embedding 与 chat 都经 async-openai」。本次移除 embedding 后，async-openai
仍被 pageindex 的 chat + tool-calling 路径使用（`chat.rs`），故依赖保留。删除的是 r6
这条**需求**（它约束了 embedding 实现方式），而非依赖本身。

### 决策 4：失效时只报错引导，不静默退化

`context_run` 在 pageindex 索引缺失/过期/损坏、或 `ChatConfig::from_env` 失败时，**必须**
返回 `quality=unavailable` + `qualityNote` 可操作提示，而非降级。提示设计（r11）：

- **言简意赅**：这些 `qualityNote` 会被下游 agent 读进上下文，冗长 = 浪费 token。
- **可操作**：明确缺哪个变量（`LLMAN_SDD_INDEX_CHAT_MODEL`）、怎么修
  （`llman sdd index rebuild`），不解释原理。

示例（目标形态，非最终文案）：
```
qualityNote: "index missing; run `llman sdd index rebuild`"
qualityNote: "LLMAN_SDD_INDEX_CHAT_MODEL unset; set a tool-calling chat model"
```

### 决策 5：`Backend` enum 的去留

pageindex 唯一后，`Backend` enum、`resolve_backend`、`backend_subdir` 等抽象可能过度。
实施时评估：
- 若 pageindex 路径不再需要「按 backend 选目录/选函数」，则移除 enum，`context_run`
  直接调 pageindex 逻辑，`.context/pageindex/` 简化为 `.context/`。
- 但为减少本次改动爆炸半径，**倾向先保留 `Backend::Pageindex` 作为单值 enum**，
  简化是后续 refactor change 的事。本 change 只删 `Rag`。

## 文件改动清单

| 文件 | 动作 | 说明 |
|------|------|------|
| `src/sdd/command.rs` | 改 | `--backend` 校验收紧为仅 pageindex |
| `src/sdd/context/mod.rs` | 改+删 | 删 rag 分支与函数；pageindex 路径加友好提示 |
| `src/sdd/context/index.rs` | 删 | 删 rag 索引结构与分级逻辑；保留 pageindex/通用部分 |
| `src/sdd/context/embed.rs` | **删** | embedding 客户端，仅 rag 用 |
| `docs/sdd-context-index.md` | 改 | 移除 rag 章节 |
| `templates/sdd/<locale>/units/skills/sdd-commands.md` | 改 | 更新 `--backend` 说明 |
| `agentdev/eval/run.ts` | 改 | 默认 variants 去 rag；rag 标 legacy |

## 测试策略

- **单元**：`--backend rag` 报错（非 panic）；缺 chat model 时提示含变量名与 rebuild 命令。
- **回归**：默认 `sdd context` 与 `--backend pageindex` 行为一致（沿用 pageindex 既有测试）。
- **eval**：pageindex 在 xylitol 100 题上 F1 ≥ 基线 `e5a96fdb`（66%），证明删除 rag 未伤及
  pageindex。

## 迁移与回退

- **迁移**：使用 `--backend rag` 的用户改为默认即可（pageindex 本就是默认）。
- **回退**：本 change 归档前若发现 pageindex 在某场景不可接受，可 revert 本 change；
  rag 代码从 git 历史恢复。
