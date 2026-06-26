# Design: Embedding 原生 Rust 化

## HTTP 客户端选型

| 方案 | 类型 | 优点 | 缺点 |
|------|------|------|------|
| `ureq` | 同步 blocking | 零依赖 tokio runtime、API 简洁、编译快 | 缺少自建连接池（但 embedding 调用为低频操作） |
| `reqwest::blocking` | 同步 blocking on async | 社区最流行、功能最全 | 隐式依赖 tokio runtime、编译时间长、体积大 |
| `attohttpc` | 同步 | 最轻量 | 更新频率低、功能有限 |

**决策：使用 `ureq`（v3，带 `json` feature）。**

理由：
- embedding API 调用是低频操作（一次 index rebuild 发 ~38 个 batch 请求），不需要连接池或异步支持。
- 现有项目中已有 tokio 依赖，但 embedding 调用是同步的，不需要引入 tokio runtime 的开销。
- `ureq` 编译快、API 直观，适合这种简单的 POST + JSON 响应的场景。

## Embedding API 协议

保持与 Python 脚本一致的 OpenAI-compatible 协议：

```
POST {api_url}/embeddings
Authorization: Bearer {api_key}
Content-Type: application/json

{
    "model": "{model}",
    "input": ["text1", "text2", ...],
    "encoding_format": "float"
}
```

响应格式：
```json
{
    "data": [
        {"index": 0, "embedding": [f32; dim]},
        {"index": 1, "embedding": [f32; dim]}
    ]
}
```

## 环境变量优先级

```
CLI arg (--api-host/--model/--api-key)
  > env LLMAN_SDD_INDEX_OPENAI_API_HOST / _KEY / _MODEL
  > hardcoded defaults
```

## 文件组织结构

新增 `src/sdd/context/embed.rs` 模块，包含：
- `embed_texts()`：核心函数，支持 batch + retry
- 私有辅助函数：`build_request()`、`parse_response()`、`batch_chunks()`

`src/sdd/context/mod.rs` 相应移除：
- `embed_query()` 中的 Python spawn 逻辑（替换为 `embed_texts()` 调用）
- `index_rebuild()` 中的 Python spawn 逻辑（替换为 `embed_texts()` 调用）
- `find_script_path()` 函数（不再需要）

## 边界情况处理

- 空文本列表：立即返回空 `Vec`
- API 返回错误状态码：ureq 的 `StatusError` 自动传播，增加错误信息上下文
- 所有 batch 均失败：聚合错误信息，不部分提交
- API URL 路径中缺少 `/embeddings`：自动补齐（与 Python 脚本行为一致）
