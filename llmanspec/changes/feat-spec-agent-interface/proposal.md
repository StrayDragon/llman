# Proposal: Spec Agent Interface — Unified `context` 命令

## Why

当前 agent 与 specs 的交互有两个核心问题：

**问题 A：信息搜寻成本高**
- `llman sdd list --specs --json` 只返回 `{id, title, requirementCount}`，agent 拿到 34 个名字后不知道 purpose、valid_scope、health
- 没有途径一次调用就获得「哪些 spec 与当前任务相关」——要么全读全文（~187KB），要么人工猜
- 纯关键词匹配实验验证了无法解决语义漂移：`XDG_CONFIG_HOME` 在所有 spec 中都不存在，`config` 又是高频噪声

**问题 B：Embedding 工程维护成本不清晰**
- Index 何时需要重建？重建时 agent 是否要等待？
- Embedding API 不可用时怎么办？回退方案是啥？
- 如果回退方案质量差、浪费更多 token，不如直接报错

本 change 不解决所有 spec 质量问题（见 `feat-spec-quality-triage`），只解决 **agent 一次调用即可获得高质量 spec 上下文** 的问题。

## What Changes

### 1. `llman sdd context` 统一命令（核心）

替代之前分散的 `query --path`、`query --keyword`、`list --meta`、`search` 方案。
**agent 只需要记住一个命令**。

```bash
llman sdd context \
  --task "add XDG_CONFIG_HOME support" \
  --paths "src/config.rs,src/path_utils.rs"
```

返回统一 JSON：

```json
{
  "status": {
    "ok": true,
    "quality": "semantic",
    "qualityNote": null
  },
  "direct": [
    {
      "id": "config-paths",
      "zScore": 3.14,
      "reason": "Config directory resolution precedence",
      "purpose": "Define how llman resolves the configuration directory...",
      "reqCount": 4,
      "health": "good",
      "staleness": "fresh",
      "scope": ["src/", "tests/"],
      "matchReqs": [
        {"id": "r1", "title": "Config directory resolution precedence"},
        {"id": "r4", "title": "Invalid config directory errors"}
      ]
    }
  ],
  "related": [
    {
      "id": "cli",
      "zScore": 0.91,
      "reason": "Config dir guard only for global-config commands",
      "health": "stale",
      "staleness": "stale"
    }
  ],
  "summary": {
    "totalSpecs": 34,
    "tierDirect": 1,
    "tierRelated": 2,
    "unrelatedCount": 29,
    "staleWarnings": ["cli"],
    "readRecommended": ["config-paths"]
  }
}
```

Agent 读一次就知道：
- `direct[0]` → 必须读全文（只 1 个）
- `related` → 可能相关，按需读
- `unrelatedCount: 29` → 不用看
- `staleWarnings` → cli 已过时，谨慎参考

### 2. 内部检索架构（Rust 实现）

```
llman sdd context --task "X" --paths "Y,Z"
    │
    ├── 1. 加载 index 元数据
    │     ├── 存在 + 新鲜 → 用 embedding 检索
    │     ├── 存在 + 过期 → keyword 检索 + quality:degraded
    │     └── 不存在 → 报错：请先运行 `llman sdd update`
    │
    ├── 2. valid_scope 预过滤（--paths）
    │     └── 34 → ~5 候选
    │
    ├── 3. embedding 语义匹配
    │     ├── embed query（调用预存模型或 API）
    │     ├── cosine sim with index 向量
    │     ├── per-requirement max-pooling → per-spec
    │     └── z-score 归一化 → tier 分类
    │
    ├── 4. 附上 health/staleness 元数据
    │
    └── 5. 返回单个 JSON
```

**层叠检索策略（Cascading Retrieval）**：

| 条件 | 检索方式 | `quality` 信号 |
|------|---------|---------------|
| Index 新鲜 + API 可达 | 语义 embedding | `"semantic"` |
| Index 新鲜 + API 不可达 | 语义 embedding（本地向量） | `"semantic"`（向量在本地） |
| Index 过期 | 纯关键词子串匹配 | `"keyword"` + `qualityNote: "Index stale"` |
| Index 不存在 | 报错 | `"unavailable"` + 指引 |

### 3. Index 维护

**存储结构**（`llmanspec/.context/`）：
```
llmanspec/.context/
  ├── metadata.toml      # version, specHash, specCount, chunkCount, buildTimestamp, model
  ├── specs.json         # spec 元数据数组
  ├── chunks.json        # 672 chunks (specId, reqId, text)
  └── vectors.bin        # f32 [n_chunks, embedding_dim] 二进制
```

**metadata.toml**：
```toml
version = 1
spec_hash = "sha256-of-all-spec-files"
spec_count = 34
chunk_count = 672
build_timestamp = "2026-06-26T05:00:00Z"
model = "bge-m3-mlx-8bit"
embedding_dim = 1024
```

**新鲜度检测**：
- `spec_hash` = sha256(所有 spec 文件按路径排序拼接)
- `llman sdd context` 调用时，对比当前 spec 文件 hash 与 metadata 中的 hash
- 匹配 → 新鲜；不匹配 → 过期

**重建时机**：
- **`llman sdd update`** 时自动重建 index（`context` 的依赖在上次 update 时准备好了）
- **`llman sdd index rebuild`** 显式重建（agent 发现 index 过期时调用）
- **永不阻塞 `context`**：`context` 只**读** index，不写

**重建方式**：
- 读取所有 spec 文件
- 解析 requirements/scenarios 为 chunk
- 调用 embedding API（或本地 fastembed）生成向量
- 写回 `llmanspec/.context/`

### 4. `list --specs --json` 扩展

```json
{
  "id": "errors-exit",
  "purpose": "Define llman CLI error rendering and exit behavior.",
  "validScope": ["src/", "tests/"],
  "requirementCount": 2,
  "health": "good",        // good | warn | stale
  "staleness": "fresh"      // fresh | stale | unknown
}
```

### 5. `llman sdd index rebuild` 子命令

```bash
# 重建 index（同步，输出进度）
llman sdd index rebuild

# 仅检查 index 状态，不重建
llman sdd index rebuild --check
# → Index: fresh (built 2026-06-26, 34 specs, 672 chunks, model: bge-m3-mlx-8bit)

# 指定 embedding API
llman sdd index rebuild --api-url http://coral:11534 --api-key "..." --model bge-m3-mlx-8bit
```

### 6. 错误处理哲学

**核心原则：从不静默回退到低质量替代。**

| 场景 | 行为 | token 成本 |
|------|------|-----------|
| Index 新鲜 + embedding 可用 | `quality: "semantic"` + 完整结果 | ~1KB |
| Index 过期 + keyword 可用 | `quality: "keyword"` + `qualityNote: "Index stale, run llman sdd update"` | ~1.2KB |
| Index 不存在 | `quality: "unavailable"` + 清晰指引 | ~0.3KB |
| 不传 `--task` 也不传 `--paths` | `quality: "unavailable"` + 提示至少传一个参数 | ~0.3KB |
| Index 损坏（无法解析） | `quality: "unavailable"` + 提示重建 | ~0.3KB |

**禁止的静默回退**：
- ❌ embedding API 挂了 → 静默切到 BM25（让 agent 以为结果是语义质量的）
- ❌ index 过期 → 静默返回过时结果
- ❌ index 损坏 → 静默重建（阻塞当前 context 调用）

### 7. Prompt/Skill 引导更新

- `llman-sdd-onboard.md`：
  > 使用 `llman sdd context --task "<what you're doing>" --paths "<files>"` 获取相关 specs。
  > 如果返回 `quality: "unavailable"`，先运行 `llman sdd update`。
- `llman-sdd-explore.md`：在「建议动作」中加 `llman sdd context` 作为首要调查手段
- `llman-sdd-quick.md`（新增）：小变更快速路径 skill（与 `context` 配合使用）

## Capabilities

- `cli`: `llman sdd context` + `llman sdd index rebuild` + `list --json --meta`
- `config-paths`: `llmanspec/.context/` 索引存储路径定义
- `sdd-workflow`: `context` 行为规范 + `index rebuild` 行为规范
- `prompts-management`: skill 模板更新（onboard / explore / quick）
- `sdd-structured-skill-prompts`: structured protocol 加检索策略约束

## Impact

- **非破坏性**：所有新增子命令，不删除现有
- **新增依赖**：需要 embedding API（coral 或 fastembed）来重建 index；context 命令本身不需要额外依赖（只读向量文件）
- **新增存储**：`llmanspec/.context/` 目录，建议 `.gitignore` 中排除向量二进制，保留 metadata.toml 用于校验
- **index 大小**：34 specs × 672 chunks × 1024 dims × 4 bytes ≈ 2.7MB（vectors.bin）

## 待定设计问题（将在 design.md 中解决）

1. **embedding API 调用方式**：`llman sdd index rebuild` 是否用 Rust HTTP client 调 coral API，还是 shell out 到 Python 脚本？
2. **`--task` 为空时的行为**：如果没传 task，只按 `--paths` 过滤 valid_scope 并返回所有匹配 spec？
3. **health 信号来源**：当前 `validate` 不输出 health，v1 暂用 staleness（git-based）代替？
