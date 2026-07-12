针对 `src/`（及相关会跑到它的测试）的审查结论如下。分支相对 base 无 diff，因此这是**全量核心代码审查**，不是 PR diff 审查。

## 总览

最值得优先处理的是三类：**配置 TOCTOU 覆盖**、**未校验的 git ref / 环境变量注入**、**若干并发文件操作竞态**。路径穿越在 SDD ID / skill slug / prompt 路径上整体防护较好。

---

## 1. 安全问题

| 严重度 | 位置 | 发现 |
|--------|------|------|
| High | `sdd/spec/staleness.rs:264-297` | `LLMANSPEC_BASE_REF` 原样进 `git` argv，无 `--` / 不以 `-` 开头校验，存在 **git option injection** |
| High | `x/claude_code/command.rs:625-628`（codex 同类） | `inject_env_vars` 把配置里所有 key 注入子进程；`env_injection` 只校验语法，**不拦** `LD_PRELOAD`/`PATH`/`DYLD_*` |
| Medium | `x/claude_code/command.rs:219-227` | SecurityChecker 仅警告，**始终继续执行** |
| Medium | `x/claude_code/config.rs`（list 脱敏） | 只按 `KEY`/`TOKEN`/`SECRET` 遮罩，`PASSWORD` 等仍可能明文打印 |
| Medium | `self_command.rs`（PowerShell `$PROFILE`） | 写入目标无 “必须在 home 下” 约束 |
| Low | `sdd/context/chat.rs:37-39` | 默认 host `http://coral:11534/v1`，外部环境可能意外外连 |

**做得好的地方：** SDD ID 校验、`validate_path_segment`（prompts）、skill slugify、账号文件 `0o600`、账号模板用 `persist_noclobber`。

---

## 2. 代码质量

- 路径校验强度不一致：`create_validated_pathbuf` 几乎只拒空串，真正安全的是 `validate_path_segment` / `validate_sdd_id`，边界用法不统一。
- 多套 `Config` 命名并存（`crate::config` / `tool::config` / `x::claude_code::config`），阅读成本高。
- `managed_block` 与 completion marker 逻辑有重复。

---

## 3. 漏洞（可利用面）

| 严重度 | 位置 | 发现 |
|--------|------|------|
| High | 同上 env inject | 恶意/共享 `claude-code.toml` 可劫持子进程加载器 |
| High | 同上 `LLMANSPEC_BASE_REF` | CI/共享 runner 上可影响 git 行为 |
| Low | 多处 `read_to_string` + YAML/TOML | 无文件大小上限，恶意超大配置可造成内存尖峰（DoS） |
| Low | `x/cursor/database.rs` | `SQLITE_OPEN_NO_MUTEX`：若连接跨线程使用则不安全（当前若单线程则风险低） |

---

## 4. 竞态条件

| 严重度 | 位置 | 发现 |
|--------|------|------|
| High | `config_schema.rs:326-351` | `ensure_global_sample_config`：`exists` 后 `atomic_write_with_mode` **可覆盖**并发写出的真实配置；同仓库已有 `atomic_write_new_with_mode` 却未用 |
| Medium | `sdd/change/archive.rs:126-134` | `exists` → `rename`，并发 archive 竞态 |
| Medium | `skills/targets/sync.rs` | symlink：`metadata` → remove → create，无排他创建 |
| Medium | `sdd/context/index.rs` | rebuild lock：PID `kill -0` 有 PID 复用窗口；非 Unix 几乎清不掉 stale lock |
| Low | `fs_utils.rs:7-28` | `persist` 若目标是 symlink 会写到链接目标；并发 last-wins |

---

## 5. 测试不稳定性

| 严重度 | 位置 | 发现 |
|--------|------|------|
| Medium | `tests/performance_tests.rs:86-90` 等 | 硬编码 wall-clock（如 `< 5000ms`），慢 CI/高负载易 flaky |
| Medium | `sdd/context/chat.rs:203-208` | 读环境变量却未走 `TestProcess`/`ENV_MUTEX`，可能误 skip 或与并行 env 测试打架 |

多数 integration 测试已用 `TempDir`/`TestEnvironment`，方向正确；上述两处是明显例外。

---

## 6. 可维护性

若干模块体量过大（约 1k–1.7k LOC），职责混杂，冲突与 review 成本高：

- `skills/cli/command.rs`、`sdd/spec/validation.rs`、`skills/targets/sync.rs`
- `x/codex/agents.rs`、`tool/sync_ignore.rs`、`usage_stats/tui.rs`

建议：CLI / 纯逻辑 / 测试拆分；统一 path/id 校验入口；Config 类型显式命名。

---

## 建议修复优先级

1. `ensure_global_sample_config` → `atomic_write_new_with_mode`
2. 校验/拒绝危险 `LLMANSPEC_BASE_REF`，argv 加 `--`
3. inject/import 侧 denylist + 复用 `is_valid_env_key`
4. archive / skills sync / rebuild lock 硬化
5. 去 flaky：perf 测试默认 skip；chat 测试用 `TestProcess`
6. 大模块拆分（可渐进）

---

接下来你更想怎么推进？可多选：

1. **按优先级修 High 项**（sample config TOCTOU + git ref + env denylist）
2. **只修竞态**（archive / skills sync / rebuild lock）
3. **只修测试不稳定性**（perf + chat env）
4. **深入某一条**：给出 exploit 草图 + 具体 patch 方案
5. **可维护性**：先拆某一个大文件的计划
6. **先暂停**，我只保留这份报告作参考
