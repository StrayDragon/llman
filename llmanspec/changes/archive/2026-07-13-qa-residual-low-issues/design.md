# Design: QA Residual Low Issues

## 1. 改动总览

| 问题 | 改动类型 | 影响范围 | 风险 |
|------|---------|---------|------|
| 默认 host | 常量修改 | `chat.rs` | 低（仅影响未配置用户的回退行为） |
| `persist` symlink | 行为加固 | `fs_utils.rs` | 低（symlink 在项目配置目录极少见） |
| 文件大小上限 | 新增辅助函数 | 多处调用点 | 中（需逐个适配调用点） |
| SQLite 标志 | 标志替换 | `database.rs` | 低（不影响当前单线程语义） |
| Config 重命名 | 纯重构 | 多处引用 | 低（纯内部改名，不暴露） |

## 2. 技术方案

### 2.1 默认 chat host

```rust
// 当前
const DEFAULT_CHAT_API_HOST: &str = "http://coral:11534/v1";

// 改为
const DEFAULT_CHAT_API_HOST: &str = "";  // must be explicitly configured
```

在 `ChatConfig::from_env()` 回退到默认值时，若为空则报错引导配置。

### 2.2 `persist` symlink 安全写入

```rust
pub fn atomic_write_with_mode(path: &Path, content: &[u8], mode: Option<u32>) -> Result<()> {
    // 在写入前检查 path 是否为 symlink
    if path.is_symlink() {
        // 方案 A：删除 symlink 后写入新文件（不跟随）
        // 方案 B：拒绝写入，返回错误
        // 推荐用 A：删除后 create_new + rename，避免竞态
        fs::remove_file(path)?;
    }
    // 后续正常 atomic write
    let tmp = ...;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

### 2.3 文件读取大小上限

```rust
/// Read file content up to `max_bytes`. Returns error if file exceeds limit.
pub fn read_with_max_size(path: &Path, max_bytes: u64) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_bytes {
        bail!("File {} exceeds size limit ({} > {})", path.display(), metadata.len(), max_bytes);
    }
    fs::read_to_string(path).map_err(Into::into)
}
```

为 YAML/TOML 解析添加统一入口：在 `crate::fs_utils` 中提供 `parse_yaml_file` / `parse_toml_file`，内部调用 `read_with_max_size`。

### 2.4 SQLite 标志

```rust
// 当前
let conn = Connection::open_with_flags(
    &db_path,
    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
)?;

// 改为（或保留 NO_MUTEX 但加安全注释）
let conn = Connection::open_with_flags(
    &db_path,
    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
)?;
// 注释：当前 cursor export 是单线程操作。若未来跨线程使用，FULL_MUTEX 保证安全。
```

### 2.5 Config 重命名

纯搜索替换：

```rust
// tool/config.rs
pub struct ToolConfig { ... }

// x/claude_code/config.rs
pub struct ClaudeCodeConfig { ... }
```

引用点用 `cargo check` 逐一排查。

## 3. 测试策略

| Task | 测试类型 | 关键断言 |
|------|---------|---------|
| 1 | 单元测试 | 默认 host 为空时 from_env 返回错误 |
| 2 | 集成测试 | symlink → atomic_write 不写入链接目标；并发写入不丢数据 |
| 3 | 单元+集成 | 超大文件返回错误；10 MiB 刚好能通过；各调用点正常 |
| 4 | 集成 | cursor export 正常导出（功能回归） |
| 5 | 编译检查 | `cargo check` 通过，无警告 |

## 4. 依赖关系

```
Task 1 (default host) ── 无依赖
Task 2 (persist) ──── 无依赖
Task 3 (size limit) ─ blocks on: Task 2（都会改 fs_utils.rs，合并处理减少冲突）
Task 4 (SQLite) ──── 无依赖
Task 5 (rename) ──── 无依赖
```

Task 2 和 Task 3 都涉及 `fs_utils.rs`，建议一起实施。

## 5. 回退计划

- Task 1–4：每个 task 独立提交，出问题回滚单个 commit
- Task 5：Config 重命名涉及文件多但纯机械替换，出问题 `git revert` 即可
