---
source_url: https://docs.anthropic.com/en/docs/claude-code/memory
title: Claude Code - Memory (CLAUDE.md)
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## Memory 的用途（原文摘录）

> Memory is how Claude Code remembers information across sessions. Memory files are text files that are included at the beginning of Claude Code conversations. They are read every time Claude Code starts up.

## Memory 类型与文件名（原文摘录）

> Claude Code has three memory locations:
>
> - Project memory: `CLAUDE.md` in the project root (shared with the project)
> - User memory: `~/.claude/CLAUDE.md` (shared across all projects)
> - Project memory local: `CLAUDE.local.md` (not committed, specific to your machine)

## 递归加载与引用（原文摘录）

> Claude Code reads memories recursively: starting from the current working directory, it walks up the directory tree and loads any `CLAUDE.md` files it finds. This allows nested subprojects to have their own memories.
>
> You can also import other files from within a memory file by using a line that starts with `#` followed by a file path.
