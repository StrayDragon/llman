# Tasks: QA Residual Low Issues

## 任务总览

| # | 任务 | 涉及文件 | 预估工作量 | 优先级 |
|---|------|---------|-----------|--------|
| 1 | 默认 chat host 加固 | `src/sdd/context/chat.rs`, `locales/app.yml` | ~10 行 | P1 |
| 2 | `persist` symlink 安全写入 | `src/fs_utils.rs`, 测试 | ~30 行 | P2 |
| 3 | 文件读取大小上限 | `src/fs_utils.rs` 新增辅助函数，各调用点适配 | ~60 行 | P3 |
| 4 | SQLite 标志加固 | `src/x/cursor/database.rs` | ~5 行 | P4 |
| 5 | Config 类型重命名 | `src/tool/config.rs`, `src/x/claude_code/config.rs` | ~50 行（搜索替换） | P5 |

## 任务详情

### [x] Task 1: 默认 chat host 加固

- **文件**: `src/sdd/context/chat.rs`
- **改动**: 将 hardcoded 默认 host `http://coral:11534/v1` 改为空字符串，host 为空时 `from_env()` 报错退出
- **校验**: 默认值不指向可路由地址；用户配置后正常使用

### [x] Task 2: `persist` symlink 安全写入

- **文件**: `src/fs_utils.rs`
- **改动**: 在 `atomic_write_with_mode` 和 `atomic_write_new_with_mode` 中检测目标是否为 symlink，若是则先删除再写入
- **测试**: symlink → `persist` 不写入链接目标；并发场景不会 last-wins

### [x] Task 3: 文件读取大小上限

- **文件**: `src/fs_utils.rs`
- **改动**: 新增 `read_with_max_size(path, max_bytes) -> Result<String>` 辅助函数 + `DEFAULT_MAX_READ_BYTES` 常量（10 MiB）
- **测试**: 正常文件正常读取；超大文件拒绝

### [x] Task 4: SQLite 标志加固

- **文件**: `src/x/cursor/database.rs`
- **改动**: 在三处 `SQLITE_OPEN_NO_MUTEX` 使用点添加 `# Safety` 注释，说明单线程只读保证
- **测试**: cursor export 功能正常

### [x] Task 5: Config 类型重命名

- **文件**: `src/tool/config.rs`, `src/x/claude_code/config.rs` 及引用点
- **改动**:
  - `tool::config::Config` → `tool::config::ToolConfig`
  - `x::claude_code::config::Config` → `x::claude_code::config::ClaudeCodeConfig`
  - 更新所有引用点
- **校验**: `cargo check` 通过，无 unused import 警告

## 验证

- [x] `cargo +nightly fmt -- --check` 通过
- [x] `cargo +nightly clippy --all-targets --all-features -- -D warnings` 通过
- [x] `cargo +nightly test` 全部通过
- [x] 每个 task 有对应测试覆盖
