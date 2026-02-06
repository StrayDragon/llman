---
source_url: https://docs.claude.com/en/docs/claude-code/settings
title: Claude Code - Settings
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## Scope 与优先级（原文摘录）

> Claude Code settings are configurable at multiple scopes.
>
> These scopes work in hierarchy; settings at higher scopes override settings at lower scopes. The precedence order is:
>
> 1. Managed settings
> 2. Command line arguments
> 3. Local project settings
> 4. Shared project settings
> 5. User settings

## 配置文件位置（原文摘录）

> | Feature | Location |
> | --- | --- |
> | User settings | `~/.claude/settings.json` |
> | Shared project settings | `.claude/settings.json` |
> | Local project settings | `.claude/settings.local.json` |
> | User agents | `~/.claude/agents/` |
> | Project agents | `.claude/agents/` |
> | Internal settings, state, and caches | `~/.claude.json` |
> | User + local MCP servers | `~/.claude.json` |
> | Project MCP servers | `.mcp.json` |
> | User + local approved tools | `~/.claude.json` |
> | Project approved tools | `.claude/settings.json` |
> | User + local plugins | `~/.claude.json` |
> | Project plugins | `.claude/settings.json` |
> | Managed settings | `managed-settings.json` |
> | Managed MCP servers | `managed-mcp.json` |
