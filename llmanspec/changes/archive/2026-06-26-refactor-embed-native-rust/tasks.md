# Tasks: refactor-embed-native-rust

## Task 1: 添加 ureq HTTP 客户端依赖

- [x] 在 `Cargo.toml` 的 `[dependencies]` 中添加 `ureq = { version = "3", features = ["json"] }`
- [x] 运行 `just build` 确认编译通过

## Task 2: 实现原生 Rust 嵌入 API 客户端

- [x] 在 `src/sdd/context/` 下创建 `embed.rs` 模块，提取 embedding API 调用逻辑
- [x] 实现 `embed_texts(texts: &[&str], api_host: &str, api_key: &str, model: &str) -> Result<Vec<Vec<f32>>>` 函数
  - 支持 batch 处理（batch_size=8）
  - 支持重试（最多 3 次，间隔 1s）
  - 使用 ureq blocking HTTP 请求
  - 解析 OpenAI-compatible embedding 响应格式
- [x] 将 `embed.rs` 注册到 `src/sdd/context/mod.rs`

## Task 3: 实现 `LLMAN_SDD_INDEX_*` 环境变量解析

- [x] 在 `src/sdd/context/mod.rs` 中实现配置解析辅助函数 `resolve_api_config()`
  - 优先级：CLI 参数 > `LLMAN_SDD_INDEX_OPENAI_API_HOST/KEY/MODEL` > 硬编码默认值
- [x] 更新 `index_rebuild()` 与 `embed_query()` 使用新的配置解析逻辑

## Task 4: 替换 `index_rebuild()` 中的 Python 调用

- [x] 移除 `index_rebuild()` 中调用 Python 脚本的代码（`Command::new("python3").arg(&python_script)`）
- [x] 替换为调用新的 `embed_texts()` 函数
- [x] 保持 index 写入逻辑不变（metadata.toml、specs.json、chunks.json、vectors.bin）
- [x] 移除 `find_script_path()` 函数

## Task 5: 替换 `embed_query()` 中的 Python 调用

- [x] 移除 `embed_query()` 中调用 Python 脚本的代码
- [x] 替换为调用新的 `embed_texts()` 函数（对单个文本查询）

## Task 6: 清理与验证

- [x] 运行 `llman sdd validate refactor-embed-native-rust --strict --no-interactive` 通过
- [x] 运行 `just check` 通过（fmt + clippy + test）
- [x] 运行 `llman sdd index rebuild` 针对真实 API endpoint 做冒烟测试（需在有 API 的环境执行）（defer → manual-smoke-test, 无外部依赖不可自动化）
