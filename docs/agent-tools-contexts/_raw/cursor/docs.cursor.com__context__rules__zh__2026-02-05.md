---
source_url: https://docs.cursor.com/zh/context/rules
title: Cursor Docs - Rules（中文）
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## 规则类型（原文摘录）

> ### Project Rules
>
> 存储在 `.cursor/rules` 目录中，并适用于整个项目。
>
> 规则以 `.mdc` 为文件扩展名，它是一种支持元数据的 MDC 格式，是 Markdown 的超集。
>
> 它是 `.cursorrules` 的替代方案。

> ### Nested Rules
>
> Nested rules allow you to have different rules for different parts of your project.
>
> Nested rules are placed in `.cursor/rules` directories within the subdirectory you want them to apply to. They are only included in the context when you are working within that subdirectory.

> ### User Rules
>
> User rules are global rules that apply to all projects.

> ### `.cursorrules`
>
> `.cursorrules` is the legacy way of adding rules.

> ### Agent: AGENTS.md
>
> Cursor supports `AGENTS.md` as a rule file.

> ### Agent: CLAUDE.md
>
> Cursor supports `CLAUDE.md` as a rule file.

## `.mdc` 规则文件格式（原文摘录）

> ### `.mdc` format
>
> The `.mdc` format includes frontmatter metadata:
>
> - `description`: A description of the rule
> - `globs`: A list of globs that determine which files the rule applies to
> - `alwaysApply`: A boolean that determines if the rule should always be applied
>
> When a rule is applied, its contents are included at the start of the model context.
