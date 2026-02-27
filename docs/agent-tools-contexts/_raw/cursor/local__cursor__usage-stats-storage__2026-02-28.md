---
source_url: local filesystem observation (~/.config/Cursor)
title: Cursor - Local State (workspace/global vscdb + bubble tokenCount, observed)
fetched_at: 2026-02-28T18:10:00+08:00
version_or_last_updated: unknown
---

# 目的

为 `agent-tools-usage-stats` 提供 Cursor 的历史/session 与 token usage 的**可解析数据源**与字段说明（只读）。

> 备注：本文件为本机 Cursor 的 SQLite 状态库结构观察（不是 Cursor 官网文档摘录）。

## 1) 关键路径（Linux 观察）

- Workspace DB（多份）：`~/.config/Cursor/User/workspaceStorage/<hash>/state.vscdb`
- Global DB（单份）：`~/.config/Cursor/User/globalStorage/state.vscdb`

macOS / Windows 的路径在本仓库已有实现参考：

- `src/x/cursor/database.rs`（`get_cursor_workspace_path()` / `get_global_db_path()`）

建议实现侧：

- SQLite 一律以只读方式打开（URI `mode=ro` 或等价）。
- v1 仅统计当前 `cwd` 对应 workspace（不做 repo/all 自动探测；v2 再扩展）。

## 2) 数据库表（schema）

观察到两个关键表（workspace/global DB 都可能存在）：

- `ItemTable(key TEXT PRIMARY KEY, value BLOB)`
- `cursorDiskKV(key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)`

global DB 中 `cursorDiskKV` schema（节选）：

```sql
CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
```

## 3) Composer：会话索引（workspace DB / ItemTable）

workspace DB 的 `ItemTable` 中存在 key：

- `composer.composerData`

其 value 是 JSON，包含：

- 顶层：`allComposers`（数组）
- 每个 composer item（字段观察，部分可选）：
  - `composerId`（session id）
  - `createdAt`（epoch ms）
  - `lastUpdatedAt`（epoch ms，可选）
  - `unifiedMode`（例如 `agent`）
  - `name`（可选）
  - `type` / `isArchived` / `isWorktree` 等

这份数据适合作为：

- sessions 列表（标题、开始/结束时间）
- 时间范围过滤（用 `createdAt/lastUpdatedAt`）

## 4) Composer：bubble 明细与 tokenCount（global DB / cursorDiskKV）

global DB 的 `cursorDiskKV` 中观察到 key 形如：

- `bubbleId:<composerId>:<bubble_uuid>`

value 为 JSON（字段观察，示例为占位）：

```json
{
  "bubbleId": "bubble_uuid",
  "type": 1,
  "createdAt": "2026-01-14T06:22:24.166Z",
  "tokenCount": { "inputTokens": 40809, "outputTokens": 6355 }
}
```

注意（v1 实现必须处理的差异）：

- `createdAt` 可能是 RFC3339 字符串，也可能缺失（`null`/不存在）。
- `tokenCount` 常见为对象 `{inputTokens, outputTokens}`；也观察到大量 bubble 为 0（实现侧仍应累加，且允许全 0）。
- 仅当 `tokenCount` 存在且为数值/对象时才计入 known-only token；不做估算。

聚合建议：

- session total：对同一 `composerId` 的所有 bubbles 累加 `tokenCount.*`。
- session start/end：
  - 优先用 `composerData` 的 `createdAt/lastUpdatedAt`（epoch ms）
  - 若 `lastUpdatedAt` 缺失，可用 bubbles 的 `createdAt`（若可用）兜底

快速验证（只读，按 composerId 查 bubbles）：

```sql
-- global db
select count(*) from cursorDiskKV where key like 'bubbleId:<composerId>:%';
```

## 5) Traditional chat（待进一步确认）

历史代码/文档中常见的 workspace key：

- `workbench.panel.aichat.view.aichat.chatdata`

但在本机观察中，workspace DB 里更多出现类似：

- `workbench.panel.aichat.<uuid>.numberOfVisibleViews`
- `workbench.panel.composerChatViewPane.<uuid>`

这意味着 “传统聊天 tabs 的稳定存储 key/格式” 可能随 Cursor 版本变化；在 v1 若无法可靠解析，应：

- 先保证 Composer 路径可用（覆盖大部分 agent 使用场景）
- Traditional chat 作为后续增量（需要更完整的 key 探测/格式适配）

## 6) 只读/安全注意事项

- 不要写入/修改 Cursor 的任何 `state.vscdb` 及其 WAL/SHM。
- 输出避免打印 bubble 的 `text` / `attached*` 等大字段（可能含敏感代码/路径）；统计只需 tokens + 时间 + id + 轻量标题。
