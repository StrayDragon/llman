# Proposal: Remove the rag backend — pageindex as the sole retrieval backend

## Why

### 问题 A：rag 检索质量在量化评测中显著落后

用冻结的 `xylitol` 语料（43 specs、100 个真实归档 change 作为标注）对三种检索变体做
了全量量化评测（报告 `agentdev/eval/results/e5a96fdb-2026-06-29T05-30-16/report.md`，
SUT = llman `e5a96fdb`）：

| variant | precision | recall | F1 | exact-direct | any-tier-recall | latency |
|---------|-----------|--------|----|--------------|-----------------|---------|
| rag | 12.2% | 80.7% | **19.2%** | 0.0% | 86.0% | 191ms |
| **pageindex** | **60.4%** | **81.3%%** | **65.5%** | **37.8%** | 79.6% | 34s |

rag 的 F1 仅为 pageindex 的约 **1/3**，根本瓶颈是 **precision 仅 12.2%**：向量相似度
把大量语义沾边但合约无关的 spec 塞进 `direct`，淹没了真正的金标。

值得注意：rag 的 any-tier-recall（86%）其实最高——它**能**召回正确 spec，只是分不清
主次。这印证了 pageindex proposal 最初的论点「similarity ≠ relevance」：spec 检索的
本质是判断「这次改动是否影响某 spec 的行为合约」，是需要推理的判断，不是相似度匹配。

### 问题 B：双 backend 制造了认知负担与维护成本

当前「pageindex 默认 + rag 可选 fallback」的设计带来：

- **用户困惑**：两个 backend 的质量差距巨大（F1 19% vs 66%），但 CLI/help 把它们并列呈现，
  暗示可互换。用户若误选 rag 会得到差得多的结果而不知。
- **代码维护**：embedding（`embed.rs`）、向量化索引（`index.rs` 的 rag 路径）、z-score
  分级等一整套逻辑只为服务一个已被证明劣于 agentic 的方案。
- **回退的错觉**：`--backend rag` 被当作「pageindex 不可用时的逃生舱」，但实测 rag 质量
  不足以承担此职责——真正的失效应报错并引导用户配置/重建 pageindex，而非悄悄退化。

### 问题 C：失效提示不够友好

当 pageindex 索引缺失或 chat 模型未配置时，现有错误提示未能简洁地告诉用户「缺什么、
怎么办」。这放大了「不如用 rag 兜底」的诱惑。

## What Changes

1. **`--backend` 仅接受 `pageindex`**（r4 改写）：移除 `rag` 作为合法值，传入非法值报错
   并引导配置。`LLMAN_SDD_INDEX_BACKEND` 仍可预设，但语义从「选择后端」变为「显式声明
   pageindex」，不存在回退。
2. **pageindex 为唯一检索后端**（r5 改写）：`status.quality` 恒为 `agentic`；失效时返回
   `quality=unavailable` + 可操作提示，**不回退**。
3. **移除 rag 专属需求**（删除 r1/r2/r3/r6）：原生 Rust embedding HTTP 客户端、embedding
   环境变量、reqwest 依赖、async-openai 替换 reqwest 这些只服务 rag 的需求一并移除。
   注：`async-openai` 仍被 pageindex 的 chat 调用使用，依赖本身保留；删除的是 r6 里
   「embedding 经 async-openai」这部分约束。
4. **新增 r11 友好可操作提示**：配置缺失/索引缺失时，提示必须言简意赅地列出环境变量名
   与 rebuild 命令，减少 token 浪费（这些提示会被下游 agent 读取）。

## Capabilities

- **`sdd-context`**：移除 rag backend、改写默认/提示语义。删除 r1/r2/r3/r6、改写 r4/r5、
  新增 r11。

## Impact

- **破坏性**：`--backend rag` 与 `LLMAN_SDD_INDEX_BACKEND=rag` 将报错。这是有意为之——
  保留它意味着保留低质量路径。迁移：用户改用默认（pageindex）即可。
- **代码删除**：`src/sdd/context/embed.rs`、`mod.rs` 的 `context_run_rag`/`index_rebuild_rag`
  及相关 rag 索引逻辑可删除；`Backend` enum 可简化为常量或移除。
- **文档**：`docs/sdd-context-index.md` 移除 rag 一节，`sdd-commands.md` 更新 `--backend`
  说明。eval harness 的 `rag` variant 可保留作历史对照（只读旧索引），但不再默认推荐。

## Evidence

- 评测报告：`agentdev/eval/results/e5a96fdb-2026-06-29T05-30-16/report.md`
- eval harness：`agentdev/eval/`（pi-agent-core + Bun，含 100 题 xylitol fixture）
- 原 pageindex proposal：`llmanspec/changes/archive/2026-06-28-feat-context-pageindex-backend/proposal.md`

## 待定问题

1. **是否物理删除 rag 代码**：本 change 的 delta 只改 spec（行为契约）。物理删除
   `embed.rs` 等是实施细节，放在 `tasks.md` 的实施阶段；是否完全删除 vs 保留为内部
   死代码，倾向**完全删除**以消除维护负担（git 历史可追溯）。
2. **eval harness 的 rag variant 去留**：保留它能持续验证「pageindex 优势」不回退，
   倾向保留（标注为 legacy）。
