---
source_url: local filesystem observation (~/.codex)
title: Codex CLI - Local State (usage stats storage, observed)
fetched_at: 2026-02-28T18:10:00+08:00
version_or_last_updated: unknown
---

# 目的

为 `agent-tools-usage-stats` 提供“Codex 会话/线程历史”的**可解析数据源**与字段说明（只读）。

> 备注：本文件为本机 `~/.codex/` 的结构观察（不是官网文档摘录）。

## 1) 关键路径（Linux 观察）

- 状态库（SQLite）：`~/.codex/state_5.sqlite`（同时可能存在 `state_*.sqlite-wal/-shm`）
- 线程回放（JSONL）：`~/.codex/sessions/YYYY/MM/DD/rollout-<ISO>-<THREAD_ID>.jsonl`

建议实现侧：

- 不要硬编码 `state_5.sqlite`：优先按 `state_*.sqlite` 选择“最大版本号”或“最新修改时间”的那个。
- SQLite 一律用只读方式打开（例如 `mode=ro` / `-readonly`）。

## 2) `threads` 表（会话/线程索引）

SQLite schema（节选）：

```sql
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    rollout_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    model_provider TEXT NOT NULL,
    cwd TEXT NOT NULL,
    title TEXT NOT NULL,
    sandbox_policy TEXT NOT NULL,
    approval_mode TEXT NOT NULL,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    has_user_event INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    archived_at INTEGER,
    git_sha TEXT,
    git_branch TEXT,
    git_origin_url TEXT,
    cli_version TEXT NOT NULL DEFAULT '',
    first_user_message TEXT NOT NULL DEFAULT '',
    agent_nickname TEXT,
    agent_role TEXT,
    memory_mode TEXT NOT NULL DEFAULT 'enabled'
);
```

与 usage stats 直接相关的字段：

- `id`：线程 id（v1 的 session id）
- `cwd`：记录的工作目录（v1：必须与当前 `cwd` **严格相等**才计入）
- `created_at` / `updated_at`：Unix epoch 秒（可作为 session start/end）
- `tokens_used`：整段线程累计 token（v1 的 known-only token 口径）
- `rollout_path`：指向对应 rollout JSONL（用于 `--with-breakdown`）
- `title` / `model_provider`：展示用（可选）

快速验证（只读）：

```bash
sqlite3 -readonly ~/.codex/state_5.sqlite \
  "select id, cwd, created_at, updated_at, tokens_used, rollout_path from threads order by updated_at desc limit 5;"
```

## 3) rollout JSONL（token breakdown）

`rollout_path` 指向 JSONL，每行一个事件对象。与 token 统计直接相关的事件（观察）：

- `type: "event_msg"`
- `payload.type: "token_count"`
- `payload.info.total_token_usage`: 会话累计（breakdown）
- `payload.info.last_token_usage`: 最近一次增量（breakdown）

示例结构（字段名级别，数值为占位）：

```json
{
  "timestamp": "2026-02-28T08:31:31.976Z",
  "type": "event_msg",
  "payload": {
    "type": "token_count",
    "info": {
      "total_token_usage": {
        "input_tokens": 123,
        "cached_input_tokens": 45,
        "output_tokens": 67,
        "reasoning_output_tokens": 8,
        "total_tokens": 190
      },
      "last_token_usage": {
        "input_tokens": 12,
        "cached_input_tokens": 4,
        "output_tokens": 6,
        "reasoning_output_tokens": 1,
        "total_tokens": 18
      },
      "model_context_window": 258400
    }
  }
}
```

解析建议：

- session 总累计：取**最后一条** `token_count` 的 `total_token_usage`（而不是求和）。
- breakdown 口径与 `threads.tokens_used`：观察到 `total_token_usage.total_tokens` 与 `threads.tokens_used` 在会话末尾通常一致；实现上仍建议以 `threads.tokens_used` 作为 v1 “known-only totals”，breakdown 作为补充显示。
- `cached_input_tokens`（如存在）应单独展示，不要把它强行并入 total（不同产品对“缓存 token 是否计费”的口径可能不同）。

快速验证（不打印正文，只找 token_count 行号）：

```bash
rg -n "total_token_usage|last_token_usage" "$(sqlite3 -readonly ~/.codex/state_5.sqlite 'select rollout_path from threads limit 1;')"
```

## 4) 只读/安全注意事项

- 不要写入/修改 `~/.codex/**`（包含 sqlite / jsonl / wal/shm）。
- 如果需要并发读取多个 rollout，建议做限速/进度反馈（文件可能很大）。
