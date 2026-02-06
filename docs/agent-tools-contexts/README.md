# Agent CLI Tools Contexts

本目录用于沉淀与「Agent CLI / IDE Agent」强相关的配置文档：包含配置方式、文件位置、解析/优先级顺序、以及与本仓库 `prompt/`、`skills/` 的对接方式。

结构约定：

- `docs/agent-tools-contexts/{claudecode,codex,cursor}/README.md`：主线结构化整理（本仓库建议的接入方式 + 关键配置点 + 优先级/解析顺序）。
- `docs/agent-tools-contexts/_raw/{claudecode,codex,cursor}/`：抓取的原始来源片段（按来源 URL 分文件保存，尽量保持原文，便于回溯）。

更新流程（建议）：

1. 先在 `_raw/<tool>/` 放入来源片段（记录 `source_url`、`fetched_at`、版本信息/发布日期）。
2. 再更新对应 `<tool>/README.md`，把“可操作的结论”结构化整理出来，并引用 `_raw` 文件路径作为证据。

备注：

- 本次更新尝试使用 DevTools 浏览器抓取外网，但在当前环境对多个站点出现 `net::ERR_TIMED_OUT`，因此改用检索工具抓取官网文本片段并落盘到 `_raw/`。
