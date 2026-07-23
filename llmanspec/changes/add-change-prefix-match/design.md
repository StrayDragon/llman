# 设计文档：Change 名前缀匹配

## 问题

当前所有 `llman sdd *` 命令中，change 名参数仅支持精确匹配，即需输入完整的 change id。用户希望输入前缀即可匹配。

## 约束

1. 向后兼容：精确匹配必须保持最高优先级（不影响现有用户）
2. 活性优先：`llmanspec/changes/`（活跃 change）优先于 `llmanspec/changes/archive/`（归档 change）
3. 不引入额外依赖（纯标准库 + anyhow）
4. 匹配失败的错误消息应包含候选建议，与现有 `nearest_matches` 风格一致

## 方案

### 核心函数

在 `src/sdd/shared/discovery.rs` 新增 `resolve_change_id`：

```rust
pub enum ChangeResolution {
    /// 唯一确定的完整 change id
    Resolved(String),
    /// 多个前缀匹配结果（用户需确认）
    Ambiguous(Vec<String>),
    /// 无任何匹配
    None,
}

pub fn resolve_change_id(root: &Path, input: &str) -> Result<ChangeResolution> {
    // 1) 精确匹配活跃 changes
    //    - 若 llmanspec/changes/<input>/proposal.md 存在 → Resolved(input)
    // 2) 前缀匹配活跃 changes
    //    - 遍历 llmanspec/changes/ 下目录名，找以 input 开头的
    //    - 若唯一 → Resolved(full_name)
    //    - 若多个 → Ambiguous(list)
    // 3) 前缀匹配归档 changes
    //    - 遍历 llmanspec/changes/archive/ 下目录名
    //    - 对每个目录提取 change id（去掉日期前缀 YYYY-MM-DD-）
    //    - 若 change id 以 input 开头
    //    - 若唯一 + 前面活跃无匹配 → Resolved(full_change_id)
    //    - 若多个 + 前面活跃无匹配 → Ambiguous(list)
    // 4) 无匹配 → None
}
```

### 集成方式

每个需要 change 名解析的命令，在入口处调用 `resolve_change_id`：

- **`show.rs`**: `show_direct` 中识别到 `type_override == Change` 或无 override 时，先 resolve
- **`validate.rs`**: `validate_direct` 中 change 分支，先 resolve 再校验
- **`status.rs`**: `resolve_target` 中，**保留**先精确/前缀匹配活跃 > 再前缀匹配归档的优先级，去除旧的 substring contains 逻辑
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
