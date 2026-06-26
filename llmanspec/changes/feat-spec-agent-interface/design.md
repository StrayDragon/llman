# Design: Spec Context Index Architecture

## 1. Index 生命周期

```
                    ┌──────────────────┐
                    │  llman sdd update │
                    │  (触发 index 重建) │
                    └────────┬─────────┘
                             │
                             ▼
┌─────────────────────────────────────────────┐
│  llmanspec/.context/ 目录                   │
│                                             │
│  metadata.toml  ← 版本 + hash + 时间戳      │
│  specs.json     ← spec 元数据 (磁盘上)       │
│  chunks.json    ← 672 个文本块 (调试用)      │
│  vectors.bin    ← f32[n_chunks, dim]        │
└─────────────────────────────────────────────┘
                             ▲
                    ┌────────┴─────────┐
                    │  llman sdd       │
                    │  index rebuild   │
                    │  (显式重建)       │
                    └──────────────────┘
                             │
                    ┌────────┴─────────┐
                    │  llman sdd       │
                    │  context --task  │
                    │  (只读 index)     │
                    └──────────────────┘
```

**关键设计**：`context` 命令是只读的，永远不会触发 index 写入。这保证了 context 的延迟稳定且可预测。

## 2. 新鲜度检测

### 2.1 spec_hash 计算

```rust
fn compute_spec_hash() -> String {
    // 1. 遍历 llmanspec/specs/*/spec.toon
    // 2. 按 spec id 排序（保证确定性的 hash）
    // 3. 拼接所有文件内容
    // 4. sha256 哈希
    let mut files: Vec<PathBuf> = /* 所有 spec.toon 文件 */;
    files.sort();
    let mut hasher = Sha256::new();
    for f in files {
        hasher.update(fs::read(f).unwrap());
    }
    hex::encode(hasher.finalize())
}
```

### 2.2 新鲜度判断逻辑

```rust
enum IndexFreshness {
    Fresh,              // spec_hash 匹配 → 可直接用
    Stale { delta: u64 }, // spec_hash 不匹配 → 建议重建
    Missing,            // .context/ 不存在
    Corrupted(String),  // metadata 解析失败
}

fn check_index_freshness(context_dir: &Path) -> IndexFreshness {
    let meta_path = context_dir.join("metadata.toml");
    if !meta_path.exists() {
        return IndexFreshness::Missing;
    }
    let meta: ContextMetadata = match toml::from_str(&fs::read_to_string(meta_path)?) {
        Ok(m) => m,
        Err(e) => return IndexFreshness::Corrupted(e.to_string()),
    };
    let current_hash = compute_spec_hash();
    if meta.spec_hash == current_hash {
        IndexFreshness::Fresh
    } else {
        IndexFreshness::Stale { delta: 0 /* 未来可算差异数 */ }
    }
}
```

### 2.3 在 context 命令中使用

```rust
fn run_context(task: Option<String>, paths: Vec<String>) -> Result<ContextOutput> {
    let context_dir = config.llman_config_dir().join(".context");

    match check_index_freshness(&context_dir) {
        IndexFreshness::Fresh => {
            // 加载 index → embedding 检索
            let index = ContextIndex::load(&context_dir)?;
            let results = index.search(task, paths)?;
            Ok(results.with_quality("semantic"))
        }
        IndexFreshness::Stale { .. } => {
            // 降级为 keyword 检索 + quality 标记
            let results = keyword_search(task, paths)?;
            Ok(results.with_quality("keyword")
                      .with_note("Index stale, run `llman sdd index rebuild` for semantic results"))
        }
        IndexFreshness::Missing => {
            // 报错，不兜底
            Err(ContextError::IndexMissing {
                hint: "No embedding index found. Run `llman sdd update` or `llman sdd index rebuild` first."
            })
        }
        IndexFreshness::Corrupted(msg) => {
            // 报错，不兜底
            Err(ContextError::IndexCorrupted {
                hint: format!("Index corrupted ({}). Rebuild with `llman sdd index rebuild`.", msg)
            })
        }
    }
}
```

### 2.4 `llman sdd index rebuild --check` 快速检查

```bash
$ llman sdd index rebuild --check
# 输出:
# Index: fresh (built 2026-06-26, 34 specs, 672 chunks, model: bge-m3-mlx-8bit)

# 或者:
# Index: stale (built 2026-06-01, current specs differ)
# Hint: rebuild with `llman sdd index rebuild`

# 或者:
# Index: missing (no embedding index found)
# Hint: run `llman sdd index rebuild`
```

即使 `--check` 也要是 **极轻量** 的——只读 metadata.toml + 算一次 hash，不加载向量。

## 3. 层叠检索（Cascading Retrieval）详细设计

### 3.1 完整路径（quality: semantic）

```
1. parse task + paths
2. load index
3. embed query (via pre-loaded model or API)
   └── 注意：query embedding 是在 context 调用时做的
       但这是 1 次 API 调用 vs index 重建是 672 次
4. cosine similarity: O(n_chunks × dim) = 672 × 1024 ≈ 688K ops
5. per-spec max-pooling
6. z-score normalize
7. classify tier: z > 0.6 → direct, z > -0.2 → related, else → skip
8. output JSON
```

### 3.2 降级路径（quality: keyword）

```
1. parse task + paths
2. extract keywords from task (split by space/camelCase, no NLP needed)
3. for each spec, count keyword hits in purpose + req statement + scenario then
4. sort by hit count, tier by threshold
5. output JSON with quality: "keyword"
```

### 3.3 不可用路径

```
直接返回错误 JSON，不执行任何检索：
{
  "status": {
    "ok": false,
    "quality": "unavailable",
    "qualityNote": "No embedding index found. Run `llman sdd update` first."
  },
  "direct": [],
  "related": [],
  "summary": { "totalSpecs": 34, "error": true }
}
```

## 4. Index 重建设计（`llman sdd index rebuild`）

### 4.1 步骘

```
1. Scan specs: llmanspec/specs/*/spec.toon
2. Parse TOON → extract purpose, reqs, scenarios
3. Build chunks: per-requirement with context
   672 chunks × ~200 chars each
4. Call embedding API in batches
   batch_size = 8 (从 coral 实验确认的安全值)
   672 / 8 = 84 batches
   Each batch ~200ms → ~17s total for full rebuild
5. Write index files
   metadata.toml: 50 bytes
   specs.json: ~10KB
   chunks.json: ~150KB
   vectors.bin: 672 × 1024 × 4 = 2.75MB
```

### 4.2 Batch Embedding 实现

```rust
// 伪代码
fn rebuild_index(api_url: &str, api_key: &str, model: &str) -> Result<()> {
    let chunks = collect_chunks()?;  // Vec<Chunk>
    let mut all_vectors = Vec::with_capacity(chunks.len() * 1024);

    for batch in chunks.chunks(BATCH_SIZE) {
        let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
        let embeddings = call_embedding_api(api_url, api_key, model, &texts)?;
        all_vectors.extend(embeddings.flatten());
        // 进度输出到 stderr（不污染 stdout）
        eprintln!("  embedded {}/{} chunks", all_vectors.len() / 1024, chunks.len());
    }

    // 写 metadata
    let metadata = ContextMetadata {
        version: 1,
        spec_hash: compute_spec_hash(),
        spec_count: specs.len(),
        chunk_count: chunks.len(),
        build_timestamp: Utc::now(),
        model: model.to_string(),
        embedding_dim: 1024,
    };
    fs::write(context_dir.join("metadata.toml"), toml::to_string(&metadata)?)?;

    // 写 vectors (binary: f32 flat array)
    let vec_bytes: Vec<u8> = all_vectors
        .chunks(1024)
        .flat_map(|v| v.iter().flat_map(|f| f.to_le_bytes()))
        .collect();
    fs::write(context_dir.join("vectors.bin"), &vec_bytes)?;

    // 写 specs.json + chunks.json
    fs::write(context_dir.join("specs.json"), serde_json::to_string(&specs)?)?;
    fs::write(context_dir.join("chunks.json"), serde_json::to_string(&chunks)?)?;

    Ok(())
}
```

### 4.3 API 调用方式

有两种方式调用 embedding API：

**Option A: Shell out to Python（v1 推荐）**

```rust
fn call_embedding_api_via_python(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    // 调用一个 Python 辅助脚本
    // (先 embed，再交回 Rust 写 index)
    let input = serde_json::to_string(&texts)?;
    let output = Command::new("python3")
        .arg("-c")
        .arg(PYTHON_EMBED_SCRIPT)
        .env("API_KEY", &api_key)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .output()?;
    let embeddings: Vec<Vec<f32>> = serde_json::from_slice(&output.stdout)?;
    Ok(embeddings)
}
```

**Option B: Rust HTTP client（v2，更集成）**

```rust
fn call_embedding_api_rust(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    // 用 ureq 或 reqwest 调 coral API
    let resp = ureq::post(&api_url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .send_json(json!({
            "model": model,
            "input": texts,
            "encoding_format": "float"
        }))?;
    let data: EmbeddingResponse = resp.into_json()?;
    Ok(data.embeddings())
}
```

**建议**：v1 用 Option A（Python 脚本，复用已验证的 coral API），v2 如果性能要求高再迁移到 Rust HTTP。

## 5. 错误处理与恢复指南（面向 agent）

### 5.1 context 命令的 JSON 约定

所有错误和非正常状态都通过 `status` 字段暴露，从不静默吞掉：

```json
{
  "status": {
    "ok": false,
    "quality": "unavailable",
    "qualityNote": "清晰的人类/agent 可读的指引",
    "errorKind": "index_missing | index_stale | api_unreachable | param_error"
  }
}
```

### 5.2 Agent 处理流程

```
1. 调用 llman sdd context --task "..." --paths "..."
2. 检查 response.status.ok
   ├── true → 用 direct/related 的结果
   └── false → 读 status.qualityNote 并按指引操作
3. 检查 response.status.quality
   ├── "semantic" → 高质量，放心用
   ├── "keyword" → 质量有限，谨慎参考
   └── "unavailable" → 停止，先修复基础设施
```

### 5.3 给 agent 的错误消息示例

| 场景 | Agent 收到 |
|------|-----------|
| Index 不存在 | `"No embedding index found. Run \`llman sdd update\` or \`llman sdd index rebuild\` first. This command requires pre-built index."` |
| API 调用超时 | `"Embedding API unreachable. Index is fresh but query embedding failed. Try again or check API availability."` |
| 参数缺失 | `"At least one of --task or --paths is required. Provide --task with a description of your change and/or --paths with file paths."` |
| Index 损坏 | `"Index corrupted (invalid metadata). Rebuild with \`llman sdd index rebuild\`."` |

## 6. 实现优先级

### Phase 1（核心路径，先可用）
1. `llman sdd index rebuild`：Python helper + Rust CLI 壳
2. `llman sdd context`：只读 index + embedding 检索
3. 新鲜度检测：spec_hash + metadata 校验
4. 层叠降级：index 不存在时清晰报错

### Phase 2（质量提升）
1. keyword 降级路径：index 过期时不报错，返回 keyword 结果
2. `list --json --meta` 扩展
3. `llman sdd index rebuild --check` 快速检查

### Phase 3（Prompt/Skill 集成）
1. skill 模板更新
2. 结构化协议增加检索策略约束
3. `llman-sdd-quick` skill

## 7. 索引存储路径规范

`llmanspec/.context/` 路径在 `config-paths` capability 中定义为：

```yaml
llmanspec/.context/
  .gitignore:        # vectors.bin 和 chunks.json 可能很大，建议 gitignore
    vectors.bin
    chunks.json
    !metadata.toml   # metadata 建议跟踪，方便其他开发者知道 index 状态和 hash
```

也可通过 `LLMAN_CONTEXT_DIR` 环境变量覆盖（与 `LLMAN_CONFIG_DIR` 模式一致）。

## 8. CLI 交互协定（Agent-Command Contract）

### 8.1 `llman sdd context`

```bash
llman sdd context \
  --task "add XDG_CONFIG_HOME support" \
  --paths "src/config.rs,src/path_utils.rs" \
  [--top 10]
```

**参数**:
- `--task <TEXT>`: 自然语言描述当前改动（可选）
- `--paths <FILES>`: 涉及的代码路径，逗号分隔（可选）
- `--top <N>`: 最大返回 spec 数，默认 10（可选）
- 至少需要 `--task` 或 `--paths` 之一

**正常返回**（index 新鲜 + quality: semantic）：

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
      "purpose": "...",
      "reqCount": 7,
      "health": "stale",
      "staleness": "stale",
      "scope": ["src/", "tests/"],
      "matchReqs": []
    },
    {
      "id": "errors-exit",
      "zScore": 0.69,
      "reason": "Config path error handling",
      "purpose": "...",
      "reqCount": 2,
      "health": "good",
      "staleness": "fresh",
      "scope": ["src/", "tests/"],
      "matchReqs": [{"id": "r4", "title": "Invalid config directory errors"}]
    }
  ],
  "summary": {
    "totalSpecs": 34,
    "tierDirect": 1,
    "tierRelated": 2,
    "unrelatedCount": 31,
    "staleWarnings": ["cli"],
    "readRecommended": ["config-paths"]
  }
}
```

**降级返回**（index 存在但过期）：

```json
{
  "status": {
    "ok": true,
    "quality": "keyword",
    "qualityNote": "Embedding index is stale. Run `llman sdd index rebuild` for semantic results."
  },
  "direct": [ /* keyword-only results */ ],
  "related": [],
  "summary": {
    "totalSpecs": 34,
    "tierDirect": 1,
    "tierRelated": 0,
    "unrelatedCount": 33,
    "staleWarnings": [],
    "readRecommended": []
  }
}
```

**错误返回**（index 不存在）：

```json
{
  "status": {
    "ok": false,
    "quality": "unavailable",
    "qualityNote": "No embedding index found. Run `llman sdd update` or `llman sdd index rebuild` first. This command requires pre-built index for semantic retrieval.",
    "errorKind": "index_missing"
  },
  "direct": [],
  "related": [],
  "summary": {
    "totalSpecs": 34,
    "error": true
  }
}
```

**参数缺失错误**:

```json
{
  "status": {
    "ok": false,
    "quality": "unavailable",
    "qualityNote": "At least one of --task or --paths is required. Provide --task with a description of your change and/or --paths with file paths.",
    "errorKind": "param_error"
  },
  "direct": [],
  "related": [],
  "summary": {
    "totalSpecs": 34,
    "error": true
  }
}
```

### 8.2 `llman sdd index rebuild`

**正常重建**:
```bash
$ llman sdd index rebuild --api-url http://coral:11534 --model bge-m3-mlx-8bit
Index: rebuilding 34 specs 672 chunks...
  embedded 64/672
  embedded 128/672
  ...
  embedded 672/672
Index: rebuilt successfully (2026-06-26, 34 specs, 672 chunks, model: bge-m3-mlx-8bit)
```

**只检查不重建**:
```bash
$ llman sdd index rebuild --check
Index: fresh (built 2026-06-26, 34 specs, 672 chunks, model: bge-m3-mlx-8bit)
```

```bash
$ llman sdd index rebuild --check
Index: stale (built 2026-06-01, 34 specs, but current specs differ)
Hint: rebuild with `llman sdd index rebuild`
```

```bash
$ llman sdd index rebuild --check
Index: missing (no embedding index found)
Hint: run `llman sdd index rebuild --api-url <URL>`
```

### 8.3 `llman sdd list --specs --json`（扩展后）

```json
[
  {
    "id": "config-paths",
    "purpose": "Define how llman resolves the configuration directory and enforces safe defaults.",
    "validScope": ["src/", "tests/"],
    "requirementCount": 4,
    "health": "good",
    "staleness": "fresh"
  },
  {
    "id": "errors-exit",
    "purpose": "Define llman CLI error rendering and exit behavior.",
    "validScope": ["src/", "tests/"],
    "requirementCount": 2,
    "health": "good",
    "staleness": "fresh"
  }
]
```

## 9. 后台重建机制（Async Rebuild）

### 9.1 为什么需要异步重建

Index 重建需要 embed 672 个 chunks，调用 ~84 次 API（batch_size=8），
估算耗时 ~30 秒。如果 agent 每次发现 index 不存在都要等 30 秒，体验很差。

策略：默认提示异步重建让 agent 继续工作，只在必须时走前台同步。

### 9.2 `index rebuild --async`

```bash
llman sdd index rebuild --async
```

行为：
1. 后台 fork/clone 进程执行 rebuild
2. 立即返回 PID
3. 重建进程将进度写入 `llmanspec/.context/.rebuild.lock`
4. 重建完成后自动删除 lock 文件，写入 metadata.toml + vectors.bin

输出：
```
Index rebuild started (PID: 12345)
Estimated 30s for 34 specs x 672 chunks
Check status: llman sdd index rebuild --check
Proceed with keyword-based discovery while building:
  llman sdd list --specs --json   # 查看所有 specs 元数据
  llman sdd context will auto-detect when index is ready
```

### 9.3 重建锁文件 `.rebuild.lock`

```toml
pid = 12345
started_at = "2026-06-26T05:00:00Z"
chunks_total = 672
chunks_done = 320
progress_pct = 47.6
```

`--check` 读取 lock 文件：
- 如果 pid 存活 → 输出进度，估算剩余时间
- 如果 pid 已死（stale）→ 输出 `rebuild failed`，清理 lock，提示重试
- 如果没有 lock 且有 metadata → 输出 `fresh`
- 如果没有 lock 且没有 metadata → 输出 `missing`

### 9.4 context 命令在 index 缺失时的行为升级

当前 context 的 `quality: unavailable` 错误消息增强为：

```
"No embedding index found. Options:
  1. Run `llman sdd index rebuild --async` in background (~30s)
     Then proceed with: llman sdd list --specs --json  (keyword metadata)
     Context will auto-detect when index is ready.
  2. Run `llman sdd index rebuild` (synchronous, wait for completion)
     Use this if you must have semantic results before proceeding.
  3. Continue without index: llman sdd list --specs --json
     You will get keyword-based spec metadata without semantic ranking."
```

### 9.5 Agent 在 skill 中的决策指引（prompt 层）

```
当 context 返回 quality: unavailable 时：
  ├─ 如果任务不要求语义级别的 spec 匹配（大多数情况）：
  │    ├─ 启动 llman sdd index rebuild --async
  │    ├─ 用 llman sdd list --specs --json 或直接读 known spec 继续工作
  │    └─ 无需等待重建完成
  │
  ├─ 如果任务必须语义匹配（跨 capability 融合、模糊需求）：
  │    ├─ 启动前台重建或显示 async 的 PID
  │    ├─ 告诉用户："Index building (PID 12345), about 30s"
  │    └─ 用 llman sdd index rebuild --check 轮询进度
  │
  └─ 如果重建失败：
       └─ 用 llman sdd list --specs --json 降级，记录失败原因
```

## 10. Agent 使用 `context` 的工作流示例

```
USER: 帮我给 llman 添加 XDG_CONFIG_HOME 支持

AGENT: (第一步，先查 context)
$ llman sdd context --task "add XDG_CONFIG_HOME support" --paths "src/"
← 返回 quality=semantic, direct=[config-paths]

AGENT: (读相关 spec 全文)
$ cat llmanspec/specs/config-paths/spec.toon
← config-paths 有 4 个 requirements 定义当前配置解析逻辑

AGENT: (结合 context 和 spec 内容给出方案)
当前 config-paths spec 定义了类似 LLMAN_CONFIG_DIR 的解析优先级，
可以新增一条 requirement 定义 XDG_CONFIG_HOME 的插入位置。

需要走完整 SDD 流程（行为合约变更）或直接改 spec（非合约）？
```
