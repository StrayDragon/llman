# Proposal: PageIndex Backend for `sdd context`/`sdd index` — 推理式检索替代向量检索

## Why

### 问题 A：向量检索与 sdd spec 的语义错配

当前 `sdd context` 走的是传统 embedding RAG（`src/sdd/context/index.rs` + `embed.rs`）：

- 把每条 requirement 拍平成 `[spec_id] purpose | statement` 的 chunk，再 `bge-m3` 向量化 + cosine 相似度。
- 但 sdd spec **本身已经是结构化树**（`Spec` IR = `overview + requirements[] + scenarios[]`，见 `spec/ir.rs`），拍平成 chunk 反而**丢失了 spec_id / req_id 的层级结构**。
- 向量检索还有固有问题：`similarity ≠ relevance`，对「实现变更 vs 行为合约变更」这类需要**推理**的判断力不从心。

### 问题 B：PageIndex 检索是 sdd 场景的天然解

[PageIndex](https://github.com/VectifyAI/PageIndex) 的核心思想：**不向量化、不切块，而是构建文档的层级树索引，让 LLM 在树上做 agentic 推理导航**（像人类翻目录）。这与 sdd spec 的结构高度契合：

- sdd spec 的「目录」天然存在（spec → requirement），**无需 LLM 抽取 TOC**（PageIndex 处理 PDF 时 90% 的工作在这里，对 sdd 都是多余的）。
- 检索 = agent 调用 `get_document_structure` / `get_spec_content` / `list_specs` 三工具，按需读取，token 效率高、可解释、可追溯。

### 问题 C：当前 HTTP 客户端是技术债

`src/sdd/context/embed.rs` 用 `reqwest::blocking` 手写了批处理、重试、response 解析。引入 pageindex 的 agentic 检索后还要手写 tool-calling loop —— 这是重复发明轮子。`async-openai` 已封装好 chat/embeddings/tool-calling/重试/streaming，应统一复用。

## What Changes

### 1. `--backend rag|pageindex` 选项（pageindex 为默认）

`llman sdd index` 与 `llman sdd context` 都新增 `--backend`：

```bash
# 建索引（默认 pageindex）
llman sdd index rebuild                    # → pageindex 树索引
llman sdd index rebuild --backend rag      # → 传统 embedding 索引
llman sdd index rebuild --backend pageindex

# 检索（默认 pageindex）
llman sdd context --task "..." --paths "..."                    # agentic 树检索
llman sdd context --task "..." --paths "..." --backend rag      # 向量检索
```

- `--backend` 可经 `LLMAN_SDD_INDEX_BACKEND` 环境变量预设。
- **默认 pageindex**：更贴合 sdd 结构化本质，且不依赖 embedding 模型在线。
- rag 保留为 fallback（embedding 服务可用、或需要快速关键词级召回时）。

### 2. PageIndex 检索实现（三块，约 200–300 行）

**A. 建树（trivial，无 LLM）** — `src/sdd/context/tree.rs`
sdd spec IR 直接映射成 `DocumentNode` 树，序列化为 `.context/pageindex/tree.json`：

```
spec(sdd-workflow)
├─ req r1  SDD 初始化脚手架
├─ req r2  SDD 指令刷新
└─ ...
```

**B. 三个检索工具** — `src/sdd/context/retrieve.rs`
对应原版 `retrieve.py` 的三个函数，语义对齐到 sdd 的 spec_id/req_id 寻址（sdd 没有「页码」概念）：

| 工具 | 作用 | 输入 → 输出 |
|------|------|------|
| `list_specs` | 文档元数据清单 | → `[{spec_id, purpose, req_count, ...}]` |
| `get_document_structure` | 树结构（去 text 省 token） | `spec_id` → 树 JSON（仅 title/req_id） |
| `get_spec_content` | 取具体内容 | `spec_id` + `req_ids[]` → `[{req_id, statement}]` |

**C. Agentic 检索 loop** — `src/sdd/context/retrieve.rs`
给 chat 模型一个 system prompt + 上述工具，让它自主推理导航，最终产出与 rag backend **输出格式兼容**的 `{direct, related, summary}` JSON。

### 3. `async-openai` 替换 `reqwest`

- `Cargo.toml`：移除 `reqwest`（仅 context 子命令用），加 `async-openai`。
- `embed.rs` → 用 `async_openai::Client` 的 `embeddings()`；保留批处理语义。
- 新增 `src/sdd/context/chat.rs`：封装 chat-completion + tool-calling loop。
- `context_run` / `index_rebuild` 改为 async（`#[tokio::main]` 或 `tokio::runtime::Runtime::new()?.block_on()`）。

### 4. 配置项分离：embedding 模型 vs chat 模型

| 环境变量 | 用途 | 默认 |
|---------|------|------|
| `LLMAN_SDD_INDEX_MODEL`（现有） | embedding（rag backend 用） | `bge-m3-mlx-8bit` |
| `LLMAN_SDD_INDEX_CHAT_MODEL`（新） | chat + tool-calling（pageindex 用） | —（需配置） |
| `LLMAN_SDD_INDEX_CHAT_API_HOST/KEY`（新） | chat 模型 endpoint | 复用 embedding 的 host/key |
| `LLMAN_SDD_INDEX_BACKEND`（新） | 默认 backend | `pageindex` |

### 5. 索引存储隔离

```
llmanspec/.context/
├── rag/                 # 传统向量索引
│   ├── chunks.json
│   ├── vectors.bin
│   ├── specs.json
│   └── metadata.toml
└── pageindex/           # 树索引
    ├── tree.json
    └── metadata.toml
```

`check_freshness` 按 `--backend` 检测对应子目录。

## Capabilities

- **`sdd-context`**（主改动）：`--backend` 选项、pageindex 检索、async-openai、chat 模型配置、索引隔离。

### 不引入新 capability 的理由

pageindex 检索是 `sdd-context` 的**实现策略**，对外 CLI 契约（`context` 命令的输入输出 JSON 结构）保持不变，只是 `quality` 字段从 `semantic` 扩展为 `semantic`/`agentic`。因此全部归入 `sdd-context` 的 delta，不新建 spec。

## 风险与回退

### 风险 1：chat 模型 tool-calling 能力不稳定
agentic 检索依赖 chat 模型的 function calling 质量。弱模型可能漏调工具或乱读内容。
**缓解**：system prompt 明确「先调 list_specs，再 get_document_structure，最后 get_spec_content」的三步导航协议；并保留 `--backend rag` 作为确定性 fallback。

### 风险 2：async 化的连锁改动
`context_run`/`index_rebuild` 改 async 会触及 `command.rs` 的调用链。
**缓解**：在 `sdd` 命令入口统一用 `Runtime::new()?.block_on()` 包裹，避免逐层 async 染色（llman 其余子命令仍是同步）。仅 context 子命令内部 async。

### 风险 3：pageindex 检索延迟 > 向量检索
agentic loop 需要多轮 LLM 调用（~3–5 次 tool call），比单次 embedding + cosine 慢。
**缓解**：对 spec 数量小的项目，`list_specs` 一次返回全部元数据，减少往返；记录 `qualityNote` 让 agent 知晓检索方式。

### 风险 4：两套索引的维护负担
用户可能同时维护 rag + pageindex 索引。
**缓解**：默认只建 pageindex；rag 仅在显式 `--backend rag` 时构建。`index check` 显示两个 backend 各自状态。

### 回退路径
若 pageindex 检索质量不达预期：`export LLMAN_SDD_INDEX_BACKEND=rag` 即全局回退，无需改代码。

## 待定问题

1. **agentic 检索的输出分级阈值**：rag 用 z-score > 0.6 为 direct；pageindex 由 LLM 直接判定 direct/related，是否需要与 rag 对齐阈值逻辑？
   - 倾向：pageindex 让 LLM 自主分类（reasoning 驱动），不强套 z-score；输出 JSON 结构对齐即可，分级语义由 `reason` 字段说明。

2. **tool-calling 的轮次上限**：agentic loop 是否需要硬上限防止失控？
   - 倾向：设 `max_tool_rounds = 8`，超限返回当前最佳结果并标记 `qualityNote`。

3. **是否复用 pageindex-rs crate**：经评估，pageindex-rs（`/pageindex-rs`）只有「PDF→TOC→树」的建索引部分，**没有实现检索**，且其 PDF/TOC/tiktoken 逻辑对 sdd 全无用。故本项目内自行实现 ~250 行的建树+检索，不引入该 crate。
   - 结论：不 fork、不依赖，自写最小实现。

4. **`list_specs` 与现有 `sdd list --specs --json` 的关系**：检索工具内的 `list_specs` 是否直接调 `sdd list`？
   - 倾向：检索工具独立实现轻量版（只读 spec frontmatter + req count），避免子进程开销；语义对齐但不复用 CLI。
