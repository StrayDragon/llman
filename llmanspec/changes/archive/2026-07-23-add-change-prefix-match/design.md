# 设计文档：Change 名前缀匹配

## 问题

当前所有 `llman sdd *` 命令中，change 名参数仅支持精确匹配，即需输入完整的 change id。用户希望输入前缀即可匹配。

## 约束

1. 向后兼容：精确匹配必须保持最高优先级（不影响现有用户）
2. 活性优先：`llmanspec/changes/`（活跃 change）优先于 `llmanspec/changes/archive/`（归档 change）
3. 不引入额外依赖（纯标准库 + anyhow）
4. 匹配失败的错误消息应包含候选建议，与现有 `nearest_matches` 风格一致

## 方案

### 共享核心（match_utils.rs）

「精确 > 前缀」的解析规则是 cli spec r112 的合约核心，MUST NOT 在多个命令间分叉。
因此把纯逻辑抽取到 `src/sdd/shared/match_utils.rs::prefix_resolve`，作为单一真相源：

```rust
pub enum PrefixOutcome<'a> {
    Single(&'a str),         // 唯一命中（精确或唯一前缀）
    Multiple(Vec<&'a str>),  // 多个前缀命中；调用方决定是错误还是合法多匹配
    None,                    // 无命中
}

/// 大小写敏感（r112 要求确定性解析）。exact > unique prefix > multi prefix > none.
pub fn prefix_resolve<'a>(input: &str, candidates: &'a [String]) -> PrefixOutcome<'a>
```

### change id 解析（discovery.rs）

`resolve_change_id` 是薄包装：list changes → `prefix_resolve`（活跃优先）→ 错误处理。
返回 `Result<String>`（多匹配/无匹配直接 `bail!`），比 design 初稿的 `ChangeResolution`
枚举更符合 Rust 习惯且调用点更简洁：

```rust
pub fn resolve_change_id(root: &Path, input: &str) -> Result<String> {
    // 1) prefix_resolve against active changes
    //    - Single → Ok(id)
    //    - Multiple → bail!("matches multiple active changes:\n...")
    //    - None → 继续
    // 2) prefix_resolve against archived changes
    //    - Single → Ok(id)
    //    - Multiple → bail!("matches multiple archived changes:\n...")
    //    - None → 继续
    // 3) 无匹配 → nearest_matches 建议 或 "not found"
}
```

### target 解析（status.rs）

`resolve_target` 复用 `prefix_resolve`，但语义不同：多匹配是合法的 `TargetResult::Multiple`
（status 命令展示多候选而非报错）。MUST NOT 保留 substring `contains` fallback（r112 MUST NOT）。
大小写敏感（与 discovery 一致，消除分叉）。

### show/validate 的 type 路径（show.rs / validate.rs）

无 `--type` override 时，**精确 spec 匹配优先**于 change 前缀解析（r112：exact > prefix），
避免 spec 名（如 `cli`）被同名前缀的 change（如 `cli-xxx`）劫持。

### 集成方式

每个需要 change 名解析的命令，在入口处调用 `resolve_change_id`：

- **`show.rs`**: `show_direct` 中，`--type change` 时 resolve；无 override 时先精确 spec 匹配，未命中再 resolve change
- **`validate.rs`**: `validate_direct` 同 show 的优先级规则
- **`status.rs`**: `resolve_target` 复用 `prefix_resolve`（多匹配为合法 `Multiple`），MUST NOT 保留 substring contains
- **`graph.rs`**: `build_seed_neighborhood` 中，先 resolve 再 BFS
- **`change/archive.rs`**: `run_with_root` 中，先 resolve 再继续
- **`change/git_native.rs`**: `run_attach` / `run_checkpoint` / `run_diff` 中，先 resolve
- **`change/finalize.rs`**: `run_finalize` 中，先 resolve
- **`authoring/delta.rs`**: 所有入口函数中，先 resolve

### 特殊处理

`change new` 不参与前缀匹配：它的参数是新 change id（待创建），不用于查询已有 change。

### 错误消息格式

```
Error: change 'c12' matches multiple changes:
  - c123-fix-bug
  - c12-update-feature
Did you mean one of these?
```

```
Error: change 'zzz' not found.
```

## 涉及的文件

| 文件 | 改动 |
|------|------|
| `src/sdd/shared/discovery.rs` | 新增 `resolve_change_id` 函数（+ 单元测试） |
| `src/sdd/shared/show.rs` | `show_direct` 中 change 分支使用 `resolve_change_id` |
| `src/sdd/shared/validate.rs` | `validate_direct` 中 change 分支使用 `resolve_change_id` |
| `src/sdd/shared/status.rs` | `resolve_target` 重构为前缀优先匹配 |
| `src/sdd/shared/graph.rs` | `build_seed_neighborhood` 使用 `resolve_change_id` |
| `src/sdd/change/archive.rs` | `run_with_root` 使用 `resolve_change_id` |
| `src/sdd/change/git_native.rs` | `run_attach` / `run_checkpoint` / `run_diff` 使用 `resolve_change_id` |
| `src/sdd/change/finalize.rs` | `run_finalize` 使用 `resolve_change_id` |
| `src/sdd/authoring/delta.rs` | 各函数使用 `resolve_change_id` |
