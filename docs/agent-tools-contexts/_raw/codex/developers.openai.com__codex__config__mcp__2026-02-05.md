---
source_url: https://developers.openai.com/codex/app/config/mcp
title: Codex - MCP
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## MCP server 配置示例（原文摘录）

> ```toml
> [mcp_servers.server-name]
> command = "uvx"
> args = ["mcp-server-git"]
> env = { "GIT_SIGNING_KEY" = "${GIT_SIGNING_KEY}" }
> ```
