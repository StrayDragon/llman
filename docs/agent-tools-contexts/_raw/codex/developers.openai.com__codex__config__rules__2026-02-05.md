---
source_url: https://developers.openai.com/codex/app/config/rules
title: Codex - Command approval rules
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## approval_policy 取值（原文摘录）

> - `untrusted`: Prompts for approval on every command.
> - `on-request`: Only prompts when you explicitly ask for it.
> - `on-failure`: Only prompts when a command fails.
> - `never`: Never prompts for approval.

## 前缀规则匹配与命令分段（原文摘录）

> A "prefix" matches the start of a command segment.
>
> The command string is split into independent segments at shell control operators, including but not limited to:
>
> - Pipes: |
> - Logical operators: &&, ||
> - Command separators: ;
> - Subshell boundaries: (...), $(...)
>
> Each resulting segment is evaluated independently.
