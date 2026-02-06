---
source_url: https://docs.cursor.com/context/rules
title: Cursor Docs - Rules（英文）
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## Rules 目录（原文摘录）

> Project rules live in `.cursor/rules`. Each rule is stored as a file and version-controlled.
>
> Subdirectories can include their own `.cursor/rules` directory scoped to that folder.

## Rule Type（原文摘录）

> Rule Type  | Description
> `Always` | Always included in the model context
> `Auto Attached` | Included when files matching a glob pattern are referenced
> `Agent Requested` | Available to the AI, which decides whether to include it (must provide a description)
> `Manual` | Only included when explicitly mentioned using @ruleName

## `.mdc` frontmatter 示例（原文摘录）

> ---
> description: RPC Service boilerplate
> globs:
> alwaysApply: false
> ---

## User rules（原文摘录）

> User rules are defined in Cursor Settings > Rules.
>
> They apply to all projects and are always included in your model context.
>
> User rules do not support MDC, they are plain text only.
