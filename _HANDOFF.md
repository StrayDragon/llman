# HANDOFF: SDD Context/Index BDD-aware Improvements

## 背景

我们刚刚完成了 `context`/`rules` 从 `llmanspec/config.yaml` 迁移到 `AGENTS.md` 的工作
（commit `7041ede`）。接下来需要审视 **SDD context/index 系统在 BDD 启用时的行为**。

## 当前架构

```
spec.toon (SSOT)
  ├── kind, name, purpose
  ├── requirements[] (MUST/SHALL)
  ├── scenarios[] (feature: true/false)
  └── valid_scope[]

      │
      ▼
sdd index rebuild ──→ pageindex/tree.json
      │                  └── 由 MainSpecDoc IR 直接构建
      │                      （仅 spec.toon 内容，不含 .feature）
      ▼
sdd context ──→ agentic loop (3 tools)
                  ├── list_specs()              → spec 列表 + purpose
                  ├── get_document_structure()  → requirement 标题
                  └── get_spec_content()        → MUST/SHALL 全文

BDD-on 时额外路径:
  solidify ──→ 从 delta op_scenarios 生成 .feature 文件
  validate ──→ 解析 .feature Gherkin + 可选的 bdd.run_command
```

## 问题分析

### 1. context/index 对 BDD 零感知

当 `config.yaml` 含 `bdd:` 段时（BDD-on 模式），spec 的行为细则从 `spec.toon` 的
`scenarios[]`（`feature: true`）转移到 `.feature` 文件中。但 pageindex 索引构建
`index_rebuild_pageindex()` 完全无视 `.feature` 文件：

```rust
// src/sdd/context/mod.rs: index_rebuild_pageindex()
// 只遍历 specs/<name>/spec.toon，忽略同目录下的 *.feature
let spec_file = spec_dir.join("spec.toon");
```

这意味着：
- **`get_document_structure()` 返回空**：BDD-on 模式下 spec.toon 的 `requirements[]` 可能已
  精简（部分 spec 在 BDD-on 迁移中），`scenarios[]` 也可能已清空（`feature: true` 的都去了
  `.feature`）。context 返回的信息非常稀疏。
- **`get_spec_content()` 找不到行为合约**：MUST/SHALL 语句在 spec.toon 中保留，但具体场景
  细节在 `.feature` 中，context 不返回。
- **agentic 推理质量下降**：LLM 只能看到骨架，看不到 Gherkin 场景中的精确行为描述。

### 2. solidify 写入 .feature 但 context 不读取

`solidify` 从 delta 的 `op_scenarios` 生成 `.feature` 文件到 `specs/<name>/` 目录下。
但是 context/index rebuild 从不读取这些文件，导致新建的 `.feature` 内容对 context 不可见。

### 3. staleness hash 忽略 .feature

`compute_spec_hash()` 只 hash `spec.toon` 文件内容，不包含 `.feature`。所以修改 `.feature`
文件不会触发 index staleness，context 仍认为索引新鲜。

```rust
// src/sdd/context/index.rs: compute_spec_hash()
// 只 hash spec.toon
let entries: Vec<PathBuf> = ... .map(|p| p.join("spec.toon"))
```

### 4. context 输出不含 feature 引用

`print_pageindex_output()` 输出的 `readRecommended` 只列出 spec id，不包含具体 feature
文件路径。用户/agent 拿到后不知道去看哪个 `.feature` 文件。

## 建议改进方向

### Phase 1: 索引增强（低风险）

- **`compute_spec_hash()` 包含 `.feature` hash**：BDD-on 时 `.feature` 文件也是 spec 的
  组成部分，修改它应触发 staleness。
- **`index_rebuild_pageindex()` 读取 `.feature` 元数据**：不解析 Gherkin 全文，但至少记录
  每个 spec 下有哪些 `.feature` 文件，存入 `tree.json` 的 `docs[].features[]` 字段。
- **`get_document_structure()` 返回 feature 列表**：让 LLM 知道 BDD 模式下的 spec 有可
  执行的 `.feature` 文件。

### Phase 2: 检索增强（中等风险）

- **`get_spec_content()` 返回 feature 场景摘要**：当 BDD 启用时，对 `.feature` 做轻量
  Gherkin 解析，提取场景的 `Given/When/Then` 作为 spec 内容的补充。
- **agentic system prompt BDD 提示**：在 `SYSTEM_PROMPT` 中告知 LLM 当 spec 有 `.feature`
  时，行为细节在 feature 文件中。

### Phase 3: 输出增强（低风险）

- **`readRecommended` 含 feature 路径**：当 spec 下存在 `.feature` 文件时，在
  `readRecommended` 中同时列出 feature 路径。
- **context 输出提示 BDD 模式**：summary 中标注哪些 spec 启用了 BDD。

## 不需要改的（已确认）

- `config.yaml` 的 `context`/`rules` 已移除 → AGENTS.md + llmanspec/AGENTS.md（已完成）
- context 后端机制不变：`pageindex` 仍然是唯一后端，`rag` 已废弃
- `solidify` 本身不需要改——它写 `.feature` 的行为是正确的
- `validate` 的 BDD fast/full mode 逻辑不变

## 相关文件清单

| 文件 | 需要改动 |
|---|---|
| `src/sdd/context/index.rs` | `compute_spec_hash()` 包含 .feature；新增 feature 元数据收集 |
| `src/sdd/context/mod.rs` | `index_rebuild_pageindex()` 读取 .feature |
| `src/sdd/context/retrieve.rs` | `SYSTEM_PROMPT` 增加 BDD 提示；`get_spec_content` 含 feature 摘要 |
| `src/sdd/context/tree.rs` | `DocEntry` 新增 `features` 字段 |
| `src/sdd/shared/validate.rs` | context 输出含 feature 引用 |
| `src/sdd/spec/validation.rs` | 无改（已有 `discover_features`） |

## Spec 影响

现有 `sdd-context` spec（`spec.toon` + 3 个 `.feature`）未涉及 BDD-awareness。
新增行为需要更新 spec：

- `sdd-context/backend-and-config.feature` + 场景：BDD 模式下索引包含 .feature
- `sdd-context/pageindex-retrieval.feature` + 场景：get_document_structure 返回 feature 列表
- `sdd-context/tree-index-and-isolation.feature` + 场景：staleness hash 含 .feature

## 优先级建议

1. **P0**: staleness hash 含 .feature（Phase 1，修复数据不一致）
2. **P1**: tree.json 记录 feature 元数据 + get_document_structure 返回 feature 列表
3. **P2**: get_spec_content 返回 feature 场景摘要
4. **P3**: 输出增强（readRecommended 含 feature 路径）
