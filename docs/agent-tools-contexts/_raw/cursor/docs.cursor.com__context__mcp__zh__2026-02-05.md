---
source_url: https://docs.cursor.com/zh/context/mcp
title: Cursor Docs - MCP（中文）
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## 配置文件位置（原文摘录）

> MCP configuration files are stored in:
>
> - Project configuration: `.cursor/mcp.json`
> - Global configuration: `~/.cursor/mcp.json`

## 模板插值（原文摘录）

> MCP supports template interpolation in configuration files:
>
> - Environment variables: `${env:VAR_NAME}`
> - User input: `${input:variable_name}`
> - Workspace folder: `${workspaceFolder}`

## JSON 示例（原文摘录）

> ```json
> {
>   "mcpServers": {
>     "server-name": {
>       "command": "npx",
>       "args": ["-y", "package", "arg1"],
>       "env": {
>         "ENV_VAR": "value"
>       }
>     }
>   }
> }
> ```

## HTTP servers 的 headers（原文摘录）

> For HTTP servers, you can also specify headers:
>
> ```json
> {
>   "mcpServers": {
>     "server-name": {
>       "url": "https://api.example.com/mcp",
>       "headers": {
>         "Authorization": "Bearer ${env:API_TOKEN}"
>       }
>     }
>   }
> }
> ```
