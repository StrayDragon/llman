# Design: PageIndex Backend

> 技术设计。与 `proposal.md` 的「为什么」互补，本文聚焦「怎么做」。

## 架构总览

```
                        llman sdd context --backend <rag|pageindex>
                                          │
                            ┌─────────────┴─────────────┐
                            │                           │
                       pageindex (默认)               rag (fallback)
                            │                           │
                   ┌────────┴────────┐          ┌───────┴───────┐
                   │                 │          │               │
              tree.json          agentic     vectors.bin    cosine
              (建树无LLM)         loop        (embedding)     sim
                   │            (chat+tools)     │               │
                   └────────┬────────┘          └───────┬───────┘
                            │                           │
                            └───────────┬───────────────┘
                                        ▼
                          统一输出 JSON {direct, related, summary}
```

两套 backend 共享：配置解析、新鲜度检测框架、输出 JSON schema、async runtime 包装。
各自独立：索引构建、检索算法、存储子目录、依赖的模型类型。

## 文件改动清单

| 文件 | 动作 | 说明 |
|------|------|------|
| `Cargo.toml` | 改 | 移除 `reqwest`，加 `async-openai`；`tokio` 已有仅需确认 features |
| `src/sdd/command.rs` | 改 | `context`/`index` 子命令加 `--backend` 参数 |
| `src/sdd/context/mod.rs` | 改 | `context_run` 按 backend 分发；async 包装 |
| `src/sdd/context/index.rs` | 改 | `index_rebuild` 按 backend 分发；索引路径隔离 |
| `src/sdd/context/embed.rs` | 改 | reqwest → async-openai；保留接口签名 |
| `src/sdd/context/tree.rs` | **新增** | spec IR → DocumentNode 树；序列化 tree.json |
| `src/sdd/context/retrieve.rs` | **新增** | pageindex 三工具 + agentic loop |
| `src/sdd/context/chat.rs` | **新增** | async-openai chat + tool-calling 封装 |

## 核心数据结构

### DocumentNode（树索引，`tree.rs`）

sdd spec 不需要 PageIndex 原版的 `start_index/end_index`（页码），改用 `req_id` 寻址：

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct DocNode {
    pub spec_id: String,
    pub purpose: String,        // spec overview
    pub reqs: Vec<ReqNode>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReqNode {
    pub req_id: String,         // r1, r2, ...
    pub title: String,
    pub statement: String,      // 含 MUST/SHALL 的完整文本
}

#[derive(Serialize, Deserialize)]
pub struct TreeIndex {
    pub version: u32,
    pub spec_hash: String,
    pub build_timestamp: String,
    pub chat_model: String,     // 记录构建时配置（仅供参考）
    pub docs: Vec<DocNode>,
}
```

### 检索输出（两 backend 共享，保持兼容）

沿用现有 `context_run` 的输出结构，仅 `quality` 取值扩展：

```json
{
  "status": { "ok": true, "quality": "agentic", "qualityNote": null },
  "direct":   [{ "id": "sdd-workflow", "reason": "agent reasoned: validates change stages" }],
  "related":  [{ "id": "cli",          "reason": "..." }],
  "summary": { "totalSpecs": 35, "tierDirect": 1, "tierRelated": 2, "toolCalls": 5 }
}
```

`quality` 取值：`semantic`（rag）/ `agentic`（pageindex）/ `unavailable`（索引缺失）。

## 三工具设计（`retrieve.rs`）

工具签名对齐 OpenAI function calling schema。sdd 用 `spec_id` + `req_ids[]` 寻址（无页码概念）：

### Tool 1: `list_specs`
```json
{
  "name": "list_specs",
  "description": "List all spec documents with metadata. Call this first to see what specs exist.",
  "parameters": { "type": "object", "properties": {} }
}
```
返回：`[{ "spec_id": "sdd-workflow", "purpose": "...", "req_count": 46 }, ...]`
实现：直接读 `TreeIndex.docs`，O(1) 内存，无 IO。

### Tool 2: `get_document_structure`
```json
{
  "name": "get_document_structure",
  "description": "Get the tree structure of one spec (titles + req_ids only, no full text to save tokens).",
  "parameters": {
    "type": "object",
    "properties": { "spec_id": { "type": "string" } },
    "required": ["spec_id"]
  }
}
```
返回：`{ "spec_id": "...", "purpose": "...", "reqs": [{ "req_id": "r1", "title": "..." }] }`
（去 `statement` 省 token —— 对应原版 `remove_fields(structure, ['text'])`）

### Tool 3: `get_spec_content`
```json
{
  "name": "get_spec_content",
  "description": "Get full statement text of specific requirements in a spec.",
  "parameters": {
    "type": "object",
    "properties": {
      "spec_id": { "type": "string" },
      "req_ids": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["spec_id"]
  }
}
```
返回：`[{ "req_id": "r1", "statement": "`llman sdd init` MUST ..." }]`

## Agentic Loop（`chat.rs` + `retrieve.rs`）

```rust
pub async fn retrieve_via_pageindex(
    tree: &TreeIndex,
    task: &str,
    paths: &[String],
    chat_cfg: &ChatConfig,
) -> Result<RetrievalOutput> {
    let client = AsyncOpenAI::new(chat_cfg.api_key.clone(), chat_cfg.api_host.clone());
    let tools = build_tool_schemas();           // 三工具 schema
    let mut messages = vec![
        system_msg(SYSTEM_PROMPT),               // 三步导航协议
        user_msg(format!("Task: {task}\nPaths: {paths:?}")),
    ];

    for round in 0..MAX_TOOL_ROUNDS {            // 上限 12（见决策 5）
        let resp = client.chat(&messages, &tools).await?;
        if let Some(tool_calls) = resp.tool_calls {
            messages.push(assistant_with_tools(resp.content, tool_calls.clone()));
            for tc in tool_calls {
                let result = dispatch_tool(&tc, tree);  // 本地执行，无网络
                messages.push(tool_result_msg(tc.id, result));
            }
        } else {
            // 模型给出最终答案 → 解析 direct/related
            return parse_final_answer(&resp.content);
        }
    }
    // 超限：返回当前最佳 + qualityNote
    Ok(RetrievalOutput::truncated())
}
```

**关键点**：
- 工具执行是**纯本地**的（读 `TreeIndex` 内存结构），只有 chat 请求走网络。
- `SYSTEM_PROMPT` 明确导航协议：`list_specs → get_document_structure(候选) → get_spec_content(确认)`，最后用 JSON 给出 direct/related。
- `MAX_TOOL_ROUNDS = 12`（见决策 5），并在超限时强制一次「禁用工具」的收尾请求以保留已读内容。防止弱模型死循环或无谓轮空。

## System Prompt（导航协议）

```
You are a spec retrieval agent for an SDD (spec-driven development) project.
Given a task and optional file paths, find which specs are relevant.

NAVIGATION PROTOCOL (follow this order):
1. Call list_specs() to see all available spec documents and their purposes.
2. For specs whose purpose seems relevant to the task, call get_document_structure(spec_id)
   to see their requirement titles (cheap, no full text).
3. For requirements that look relevant, call get_spec_content(spec_id, req_ids) to read
   the full MUST/SHALL statements.
4. Finally, output ONLY a JSON object (no other text):
   {
     "direct":  [{ "id": "<spec_id>", "reason": "<one sentence why this MUST be read>" }],
     "related": [{ "id": "<spec_id>", "reason": "<one sentence>" }]
   }
- "direct" = specs whose behavior contract is affected by this change.
- "related" = specs that provide useful context but won't change.
- Be precise: prefer fewer, well-reasoned entries over many guesses.
```

## 配置解析（`index.rs` 扩展）

```rust
struct BackendConfig {
    backend: Backend,                  // Pageindex | Rag
    embed_host/key/model: String,      // rag 用（现有）
    chat_host/key/model: String,       // pageindex 用（新，回退到 embed 的）
}

enum Backend { Pageindex, Rag }

fn resolve_chat_config(embed: &ApiConfig) -> ApiConfig {
    ApiConfig {
        api_host: env("LLMAN_SDD_INDEX_CHAT_API_HOST").or(embed.api_host),
        api_key:  env("LLMAN_SDD_INDEX_CHAT_API_KEY").or(embed.api_key),
        model:    env("LLMAN_SDD_INDEX_CHAT_MODEL")
                      .expect("chat model required for pageindex backend"),
    }
}
```

## async 化策略（最小侵入）

llman 其余子命令是同步的。**只在 context 子命令入口**用 block_on 包裹，避免 async 染色扩散：

```rust
// src/sdd/command.rs (Context 分支)
fn run_context(args: ContextArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(context_run(args))   // context_run 改 async
}
```

`async-openai` 本身是 async，embed 也顺势改 async，但对外签名不变。

## 索引存储布局

```
llmanspec/.context/
├── rag/
│   ├── chunks.json
│   ├── vectors.bin
│   ├── specs.json
│   └── metadata.toml          # spec_hash, model=bge-m3, embedding_dim
└── pageindex/
    ├── tree.json              # TreeIndex 序列化
    └── metadata.toml          # spec_hash, chat_model（记录用）
```

`check_freshness(context_dir, specs_dir, backend)` 接受 backend 参数，只检测对应子目录。

向后兼容：检测到旧布局（`.context/metadata.toml` 直接在 `.context/` 下）时，视为 `rag` backend 的遗留索引，迁移到 `.context/rag/`。

## 实施决策记录（实施时补充）

1. **建树数据源用 IR `MainSpecDoc`，而非 parser 的 `Spec`**：parser 的 `Spec`/`Requirement` 在 `convert_main_doc_to_spec` 中把 `RequirementEntry { req_id, title, statement }` 压成仅 `text`（statement），丢失了 `req_id` 与 `title`。现有 rag `index_rebuild` 用 `split(':')` 黑魔法从 `text` 里抠 req_id，对 pageindex 树（需要 req_id + title + statement 三个字段）不可用。故 `tree.rs` 直接消费 spec IR `MainSpecDoc`（经 `BACKEND.parse_main_spec` 获得），`DocNode.spec_id` 用 spec 目录名（与 rag 的 `spec_id = dir name` 保持一致，确保两 backend 检索 ID 可比）。`build_tree(docs: &[(String, MainSpecDoc)])` 为纯函数，IO 在 `mod.rs`。

2. **agentic loop 用「自带消息/工具类型 + 原生 `async fn in trait` 的 `ChatInvoker`」而非 `dyn`**：为使 loop 单元测试可在无网络环境下用 mock client 跑（本环境无 embedding/chat API），`retrieve.rs` 定义轻量 `Msg`/`ToolSchema`/`ToolCall`/`ChatTurn`，loop 泛型于 `I: ChatInvoker`（trait 用 edition 2024 稳定的原生 `async fn`，无需 `async-trait` crate）。真实 async-openai 调用封装在 `chat.rs` 的 `OpenAiInvoker`（impl `ChatInvoker`），由 `context_run` pageindex 分支构造后注入 loop。

3. **`check_freshness` 按 backend 读不同 hash 源**：rag 读 `metadata.toml`（`ContextMetadata.spec_hash`），pageindex 读 `tree.json` 顶层 `spec_hash`（直接 JSON 解析，不依赖 `tree.rs` 类型，避免 Phase 1/2 模块顺序耦合）。

4. **旧布局兼容用「读时回退」而非物理搬迁**：`resolve_backend_dir(Rag)` 当 `.context/rag/metadata.toml` 不存在但 `.context/metadata.toml`（旧平铺布局）存在时，返回 `.context/` 本身作为 rag 目录（只读兼容），避免在读路径上产生意外的文件搬移副作用；下次 `index rebuild --backend rag` 自然写入新 `.context/rag/`。仅 rag backend 享受回退，pageindex 始终 `.context/pageindex/`，故 r10 隔离语义不被破坏。

5. **轮次上限从 8 提升至 12，并在超限时强制一次「无工具」收尾**：实施时实测 deepseek-chat 这类工具调用模型对 validate 类任务会顺序调用 `list_specs → get_document_structure(多 spec) → get_spec_content(多 spec)`，合法导航常需 6–8 轮（每轮可含多个 tool call，总 tool call 数 10+），`MAX_TOOL_ROUNDS=8` 偏紧会导致模型仍在探索阶段就被截断，丢失已经读到的内容（截断路径此前返回空 direct/related）。改为：(a) `MAX_TOOL_ROUNDS=12` 给模型足够回合完成三步导航协议；(b) 触发上限时不是直接丢空，而是再发一次「禁用工具」的收尾请求，强制模型基于已读内容给出 direct/related 分类，最大化保留检索成果。截断标记 `truncated=true` 仍写入 `qualityNote` 以便观测。这是对原 design.md「`max_tool_rounds = 8`」的偏差（已更新 design.md「Agentic Loop」节）。

6. **可选 debug 轨迹**：设 `LLMAN_SDD_INDEX_DEBUG=1` 时，agentic loop 把每轮 content/tool_calls 与最终结果输出到 stderr，便于观测工具调用质量与定位模型漏调工具等问题（对齐 proposal 风险监控点「记录 toolCalls 数」与「弱模型漏调工具」）。

## 实施阶段（与 tasks.md 对应）

1. **Phase 1 — 基建**：Cargo 依赖迁移、`--backend` 参数、配置项、async 包装、索引路径隔离。此阶段末两 backend 都能「检测到索引缺失」并报对错误。
2. **Phase 2 — pageindex 建树**：`tree.rs` + `index rebuild --backend pageindex`。无 LLM，可独立验证。
3. **Phase 3 — pageindex 检索**：`chat.rs` + `retrieve.rs` + 三工具 + agentic loop。`context --backend pageindex` 端到端跑通。
4. **Phase 4 — 默认切换与清理**：默认值改 pageindex、旧索引迁移、文档/skill 更新（`sdd-commands.md` 加 `--backend` 说明）。

## 测试策略

- **tree.rs**：单元测试，给定 mock `Spec` IR → 验证 `TreeIndex` 结构正确。
- **retrieve.rs 三工具**：单元测试，给定 `TreeIndex` + 工具调用参数 → 验证返回内容。
- **agentic loop**：用 mock LLM client（预设 tool_calls 序列）验证 loop 控制流、轮次上限、最终解析。
- **端到端**：在 llman 自己的 `llmanspec/specs/` 上跑 `sdd context --task "add a new validation rule" --backend pageindex`，确认返回 `sdd-workflow` 在 direct。
- **回归**：`--backend rag` 行为与改动前完全一致。
