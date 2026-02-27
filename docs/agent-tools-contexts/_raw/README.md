# Raw Sources

这里存放从官网/仓库文档抓取的原始片段（尽量保持原文），用于主线文档的证据与追溯。

约定：

- 每个文件顶部包含：
  - `source_url`
  - `fetched_at`（ISO-8601）
  - 如能识别：`version` / `last_updated` / `commit`
- 内容以“与配置/解析顺序相关”的段落为主，避免整站镜像。

补充（本仓库约定）：

- 若官方未公开但实现需要（例如 usage stats 的本地状态/索引格式），允许在 `_raw/<tool>/` 下加入**本机只读观察**笔记：
  - `source_url` 统一标注为 `local filesystem observation (...)`
  - 必须避免写入任何 secrets/对话正文；只记录路径模式、schema、字段名、以及只读验证命令
  - 文件名建议以 `local__...__YYYY-MM-DD.md` 开头，便于与抓取来源区分
