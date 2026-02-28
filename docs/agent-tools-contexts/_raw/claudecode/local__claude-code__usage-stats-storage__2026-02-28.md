---
source_url: local filesystem observation (~/.claude)
title: Claude Code - Local State (sessions + usage tokens, observed)
fetched_at: 2026-02-28T18:10:00+08:00
version_or_last_updated: unknown
---

# 目的

为 `agent-tools-usage-stats` 提供 Claude Code 的 session 历史与 token usage 的**可解析数据源**与字段说明（只读）。

> 备注：本文件为本机 `~/.claude/` 的结构观察（不是官网文档摘录）。

## 1) 关键路径（Linux 观察）

- Projects 根目录：`~/.claude/projects/`
- 每个项目一个目录：`~/.claude/projects/<project_key>/`
  - `sessions-index.json`：会话索引（用于快速枚举/时间过滤）
  - `<sessionId>.jsonl`：会话事件流（usage 在其中）
  - `<sessionId>/`：同名目录可能存在（附件/中间产物，非必需）

`<project_key>` 的命名（观察）：

- 类似把绝对路径做 “路径转义/slug” 得到的目录名（例如用 `-` 连接各段）。
- 不建议从目录名反推路径；应以 `sessions-index.json` 的 `originalPath` / entry 的 `projectPath` 为准。

## 2) `sessions-index.json`（索引）

结构（字段级别，值为占位）：

```json
{
  "version": 1,
  "entries": [
    {
      "sessionId": "uuid",
      "fullPath": "/home/.../.claude/projects/<project_key>/<sessionId>.jsonl",
      "fileMtime": 1769485852882,
      "firstPrompt": "...",
      "summary": "...",
      "messageCount": 15,
      "created": "2025-12-31T08:49:33.622Z",
      "modified": "2025-12-31T09:27:04.371Z",
      "gitBranch": "main",
      "projectPath": "/abs/project/path",
      "isSidechain": false
    }
  ],
  "originalPath": "/abs/project/path"
}
```

与 usage stats 直接相关：

- `entries[].sessionId`：v1 的 session id
- `entries[].projectPath`：项目路径（v1：与当前 `cwd` 严格相等才计入）
- `entries[].created` / `modified`：RFC3339（可作为 session start/end）
- `entries[].isSidechain`：是否 sidechain/subagent（v1 需要能单独计数 + 合计）
- `entries[].fullPath`：jsonl 路径（读取 usage）

健壮性注意：

- 观察到 `sessions-index.json` 可能出现 `fullPath` 指向的文件已不存在的情况；实现侧需 `exists()` 检查，缺失则跳过并计数。

## 3) session JSONL（usage 来源）

每行是 JSON 对象，包含多种事件类型；其中与 token 统计相关的是：

- 顶层：`timestamp`（RFC3339，Z 时区）
- 顶层：`sessionId` / `cwd` / `gitBranch` / `isSidechain`
- `message.usage`（当存在时）：token 计数

观察到的 usage 字段（不保证齐全）：

- `input_tokens`
- `output_tokens`
- `cache_read_input_tokens`
- `cache_creation_input_tokens`
- 其他：`cache_creation` / `server_tool_use` / `service_tier`（多为非数值/标记字段）

示例结构（字段级别）：

```json
{
  "timestamp": "2026-02-28T08:49:33.622Z",
  "sessionId": "uuid",
  "cwd": "/abs/project/path",
  "isSidechain": false,
  "message": {
    "role": "assistant",
    "usage": {
      "input_tokens": 1497,
      "output_tokens": 125,
      "cache_read_input_tokens": 31232
    }
  }
}
```

解析建议：

- v1 口径：只累计“明确给出”的 token（known-only）；缺失 usage 的行不做估算。
- 推荐按 session 聚合：遍历 jsonl，累加 `message.usage.*_tokens`（数值字段）。
- session 时间：优先使用 index 的 `created/modified`；必要时可用 jsonl 的最早/最晚 `timestamp` 兜底。

## 4) sidechain 处理（v1 需求）

- 默认包含 sidechain，但在展示上要拆分：
  - primary totals
  - sidechain totals
  - overall totals
- 提供开关排除 sidechain（例如 `--no-sidechain`）。

## 5) 只读/安全注意事项

- 只读打开 `~/.claude/projects/**`；不要写入任何索引/缓存（v1 尽可能无状态）。
- 输出与日志避免打印 `firstPrompt` / 完整对话内容（可能包含敏感信息）；统计只需 tokens + 时间 + id + 轻量标题。
