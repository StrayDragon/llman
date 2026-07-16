# `sdd context` Retrieval Eval (Pi + Bun)

量化对比 `llman sdd context` 的 pageindex 后端与一个 **Pi agent-core 参考检索器**
（pi-retriever），验证「手写最小 agentic loop」是否还有提升空间。

> 本 harness **不使用** promptfoo 的 `anthropic:claude-agent-sdk`（太重），
> 改用 `@earendil-works/pi-ai` + `@earendil-works/pi-agent-core` 直驱。

## 要回答的问题

1. `pageindex` 是否**稳定**（LLM 非确定性，多次跑结果一致吗）？
2. Rust 里手写的 pageindex loop，换成 **pi-agent-core** 这套成熟 runtime 会更好吗？

## 为什么是「可量化」的

核心指标全部基于**集合匹配**，不靠主观打分：

| 指标 | 含义 |
|------|------|
| `direct_precision` | 预测为 direct 的 spec 里，命中金标的占比 |
| `direct_recall` | 金标 direct 的 spec 里，被预测出的占比 |
| `direct_f1` | 二者调和 |
| `exact_direct_match` | direct 集合与金标完全一致（0/1） |
| `any_tier_recall` | 金标 direct 出现在 direct∪related 即算召回（容忍分错层） |
| `unavailable_rate` | `quality=unavailable`（索引/配置错误）的比例 |
| `truncated_rate` | pageindex 触发轮次上限的比例 |
| `tool_calls` | pageindex 平均工具调用次数（成本代理） |
| `latency_ms` | 单查询墙钟 |
| `stability` | 同一 case 多次重复，direct 集合一致的占比（仅非确定性变体） |

## 金标从哪来（关键设计）

每个**已归档的 SDD change** 就是一条免费标注：

- **task** ← change 的 `proposal.md`（提取标题 + Why 摘要）。
- **gold direct** ← 该 change 在 `specs/<id>/spec.toon` 留下的 **delta**
  （合约被改动的 spec，正是 `direct` 该返回的）。
- **gold related** ← proposal 里提及但未改动的 spec（启发式，可人工修订）。

### 默认语料库：冻结的 `xylitol` fixture（100 题）

为了**避免题库与被测对象（llman 自身）耦合漂移**，默认评测语料是一个**冻结的快照**
（`fixtures/xylitol/`），取自独立项目 `xylitol`（43 specs、100 个带 delta 的归档 change、
gold 覆盖全部 46 个 spec、含 15 道多-spec 交叉题，最大一题改动 11 个 spec）。

> 来源/时间/git sha 见 `fixtures/xylitol/SNAPSHOT.md`。快照只冻结**考卷与答案**
> （`specs/` + `changes/archive/`），**不冻结索引**——索引每次由当前 llman 二进制重建，
> 这才是被测对象。所以 llman 怎么迭代都不影响基准可比性。

llman 自己的 archive（8 题）可用 `--project ../..` 评测；其他项目用 `--project <root>`。
生成的 `cases.json` 是**可人工编辑**的，标注不确定的 case 可直接删/改。

## 变体（system under test）

| 变体 | 实现 | 同一棵树? | 同一 system prompt? |
|------|------|-----------|---------------------|
| `pageindex` | `llman sdd context --backend pageindex`（Rust，手写 loop） | ✅ `tree.json` | ✅ 镜像 |
| `pi-retriever` | 本 harness 用 pi-agent-core 重写的 loop | ✅ 同一 `tree.json` | ✅ 同一 prompt |

`pi-retriever` 与 `pageindex` 唯一变量是 **agent runtime**（pi 的重试/并行工具/
停止逻辑 vs Rust 手写 12 轮 + salvage），其余完全对齐 → 直接检验「loop 实现质量」。

## 运行

```bash
cd agentdev/eval
bun install

# 1) 从冻结的 xylitol 快照生成金标用例（无需 API）
bun run run.ts gen --fixture xylitol --cases cases-xylitol.json
#   → 100 题；可人工修订

# 2) 跑评测（需 API）；索引会在 fixture 内自动重建
bun run run.ts run \
  --fixture xylitol --cases cases-xylitol.json \
  --variants pageindex,pi-retriever --repeat 3
#   → results/<ts>/{results.json, summary.md}

# 只看计划不调 API
bun run run.ts run --fixture xylitol --dry

# 评测 llman 自身的语料（会随 llman 迭代漂移，仅用于快速检查）
bun run run.ts run --project ../.. --variants pageindex,pi-retriever
```

### 环境变量

| 变量 | 用途 | 默认 |
|------|------|------|
| `LLMAN_BIN` | llman 二进制路径 | `../../target/debug/llman` |
| `LLMAN_CONFIG_DIR` | 隔离配置目录 | `../../artifacts/testing_config_home` |
| `LLMAN_SDD_INDEX_CHAT_API_HOST/KEY/MODEL` | pageindex + pi-retriever 的 chat 模型 | **必填** |
| `JUDGE_MODEL` | （可选）pi-ai 评判 reason 文本质量，如 `openai/gpt-4.1-mini` | 不设则跳过评判 |

`pi-retriever` 复用 `LLMAN_SDD_INDEX_CHAT_*`：与 Rust pageindex 走**同一端点同一模型**，
确保对比公平。

## 文件结构

```
agentdev/eval/
├── fixtures/xylitol/          # 冻结语料（考卷+答案，随仓库提交）
│   ├── llmanspec/{specs,changes,config.yaml}
│   └── SNAPSHOT.md            # 来源/sha/更新方法
├── gen-from-archive.ts        # 金标用例生成（归档 → cases.json）
├── lib/types.ts               # Case / Gold / Output / Metric 类型
├── lib/metrics.ts             # P/R/F1 等纯函数（自带自测）
├── lib/llman.ts               # 调用 llman 二进制 + 解析输出 JSON
├── lib/pi-retriever.ts        # pi-agent-core 参考检索器（第 3 变体）
├── lib/report.ts              # 报告生成（Markdown summary + findings）
└── run.ts                     # CLI 编排（自动建索引）
```
