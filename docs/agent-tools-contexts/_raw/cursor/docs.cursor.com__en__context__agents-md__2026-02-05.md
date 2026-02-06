---
source_url: https://docs.cursor.com/en/context
title: Cursor Docs - Context（英文，含 AGENTS.md 说明）
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## `.md` / `.mdc`（原文摘录）

> Cursor supports .md and .mdc extensions. Use .mdc files with frontmatter to specify description and globs for more control over when rules are applied.

## AGENTS.md 约束（原文摘录）

> AGENTS.md is a simple markdown file for defining agent instructions. Place it in your project root as an alternative to .cursor/rules for straightforward use cases.
>
> Unlike Project Rules, AGENTS.md is a plain markdown file without metadata or complex configurations.
>
> Root level only: AGENTS.md must be placed in your project root (v1.5)
>
> No scoping: Instructions apply globally to your project
>
> Single file: Unlike .cursor/rules, you cannot split instructions across multiple files
>
> Nested AGENTS.md support in subdirectories is planned for v1.6.
