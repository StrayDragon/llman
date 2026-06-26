# Proposal: Spec Search RAG — 双语语义搜索与成熟方案评估

## Why

`feat-spec-agent-interface` 提供了结构化检索（keyword/path），但有两个边界：

1. **关键词漂移**：用户查「better error messages」搜不到 spec 里写的「错误渲染与退出行为」
2. **双语需求**：用户/agent 中文提问「配置路径怎么解析的」，spec 是中文但关键词不完全匹配；英文提问 "how does config path resolution work" 也需要能命中中文 spec 内容
3. **排序能力**：简单子串匹配无法区分「强相关」和「弱相关」的结果

需要一个轻量 RAG 系统来解决。但**不要重复造轮子**——评估 crates.io 上已有的成熟方案。

## 约束

- **中英双语**：specs 中文为主 + 英文标识符/命令名
- **轻量**：二进制增 < 5MB（不含模型下载），模型下载 < 200MB
- **本地运行**：无网络依赖（推理时）、无外部服务
- **许可兼容**：MIT / Apache-2.0 兼容 llman (MIT)

## 开源方案详细评估

### A. BM25 族（纯词法检索）

#### A1. tantivy (MIT) — ⭐ 推荐

- **定位**：Rust 生态最成熟的全文搜索引擎，Lucene-like API
- **版本**：v0.26.1
- **许可**：MIT ✅
- **特性**：BM25 排序、增量索引、分面搜索、支持 CJK 分词器（CjkTokenizer，字符级 bigram）
- **中英支持**：内置 `CjkTokenizer` 对中日韩字符做 bigram 切分；英文默认 whitespace + stemmer。对混合中英文文本可用
- **二进制影响**：~500KB-1MB（开启 mmap/lz4/stopwords）
- **冷启动**：需先建索引（`llman sdd update` 时或首次 `search` 触发）
- **成熟度**：Quickwit 公司维护，生产级
- **适配工作**：需要写索引 schema + 中文 tokenizer 配置

**结论**：如果选 BM25 路线，tantivy 是不二之选。

#### A2. bm25_turbo (AGPL-3.0) ❌

- **定位**：声称最快的 BM25 引擎，28K QPS 8.8M 文档
- **许可**：AGPL-3.0 ❌ — 与 llman MIT 不兼容（除非单独分发）
- **特性**：SIMD 加速、可选 ANN、分布式能力
- **中英支持**：需要外部 tokenizer，不内置
- **结论**：许可不兼容，排除。

#### A3. fst (MIT) — 不适合

- **用途**：有限状态转换器，用于模糊匹配和自动补全
- **不适合**：不是 BM25 排序引擎，不支持评分

### B. Embedding 族（语义检索）

#### B1. fastembed (Apache-2.0) — ⭐ 推荐

- **定位**：本地 embedding 生成，基于 Candle/ORT 推理
- **版本**：v5.17.2
- **许可**：Apache-2.0 ✅
- **模型**：支持 `intfloat/multilingual-e5-small` (118MB) — 中英双语 ✅；也支持 `jina-embeddings-v2-small-zh` (137MB)
- **性能**：MLNX 单核 ~10ms/query（model: multilingual-e5-small）
- **依赖**：Candle 或 ONNX Runtime（ORT 模式自动下载 binaries）
- **二进制影响**：~5MB（ORT binaries）+ 模型首次下载
- **索引存储**：不提供向量存储，需要自己存（简单 JSON 文件即可——34 specs ~1MB vectors）
- **中英支持**：`multilingual-e5-small` 在 MTEB 中文任务上表现良好 ✅
- **成熟度**：4.3K stars，活跃维护

**结论**：如果要语义搜索，fastembed 是最轻量的选择。

#### B2. candle (Apache-2.0) — 过低级

- **定位**：ML 框架，非 embedding 专用库
- **结论**：需要自己写推理代码，过于底层

### C. 混合方案（BM25 + Embedding）

#### C1. ir-search (MIT) — ⚠️ 过重

- **定位**：本地 Markdown 语义搜索，混合 BM25+向量+LLM 重排序
- **版本**：v0.15.0
- **许可**：MIT ✅
- **特性**：开箱即用的 Markdown 搜索
- **问题**：依赖 `llama-cpp-2`（需要本地 LLM），太重。我们的场景不需要 LLM 重排序
- **结论**：功能过剩，依赖过重。

## 推荐方案

### Tier 0（无模型，立即可用）
- `feat-spec-agent-interface` 中的 `query --keyword` + `query --path`
- 纯子串匹配 + 路径前缀匹配
- **零依赖**，毫秒级，适合 187KB 语料

### Tier 1（BM25，tantivy）
- 如果 Tier 0 的召回率不足，添加 `llman sdd search <query>` 基于 tantivy
- 索引在 `llman sdd update` 时增量构建
- 中文用 `CjkTokenizer`（bigram），英文用默认 tokenizer
- 需要 tantivy v0.26.1 依赖

### Tier 2（语义搜索，fastembed）
- 如果双语语义匹配仍不足，添加 `llman sdd search --semantic <query>`
- 使用 `intfloat/multilingual-e5-small` 模型
- 向量存 `llmanspec/.vectors/specs.json`（~1MB）
- 需要 fastembed v5.x + 模型首次下载

### 搜索策略演进路线

```
v0 (当前)     vi 全读 / 手动猜        → token 浪费
v1 (Tier 0)   query --keyword/path   → 精准匹配，零依赖
v2 (Tier 1)   search (BM25)          → 排序召回，tantivy
v3 (Tier 2)   search --semantic      → 双语语义，fastembed
```

每一层**增量叠加**，不互相替代——agent 先试 `query --keyword`，不足再 `search --semantic`。

## Capabilities

- `cli`: `llman sdd search` 子命令
- `sdd-workflow`: search 行为的规范约束
- `config-paths`: search 索引存储路径

## Impact

- **非破坏性**：新增子命令，不影响现有命令
- **Tier 1 新增依赖**：`tantivy = "0.26.1"`（MIT）
- **Tier 2 新增依赖**：`fastembed = "5.17.2"`（Apache-2.0）
- **索引存储**：`llmanspec/.search/`（用户可 `.gitignore`，但建议跟踪——共享给其他开发者加速首次搜索）

## 待定问题

1. **Tier 1 与 Tier 2 的实现顺序**：是否先做 tantivy 再做 fastembed，还是直接跳到 fastembed？
   - 建议：先 Tantivy（低依赖、纯 Rust），使用中发现语义不足再叠加 fastembed
2. **索引刷新策略**：每次 `llman sdd update` 全量重建 vs 增量更新？
   - 建议：全量重建（34 specs 187KB，重建 < 50ms）
3. **搜索结果的 chunk 大小**：返回整个 requirement 还是单个 sentence？
   - 建议：按 requirement block 切分（每个 req 是自包含的语义单元）
