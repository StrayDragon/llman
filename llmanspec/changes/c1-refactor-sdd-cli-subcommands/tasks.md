# Tasks: Refactor `llman sdd` CLI Subcommands

## Phase 1: 添加 `status` 命令

- [x] 1.1 在 `SddCommands` 枚举中添加 `Status` 变体
- [x] 1.2 实现 `status::run()` 函数
- [x] 1.3 添加单元测试
- [x] 1.4 更新 help 文档
- [x] 1.5 验证：`llman sdd status --help` 正确显示

## Phase 2: 统一 `spec` 和 `delta` 命名

- [x] 2.1 重命名 `spec add-requirement` → `spec add-req`
- [x] 2.2 拆分 `delta add-op` 为独立子命令
- [x] 2.3 保留 `delta add-op` 作为 deprecated alias
- [x] 2.4 更新 help 文档和示例
- [x] 2.5 验证

## Phase 3: 重组非核心命令

- [x] 3.1 创建 `SddProjectCommands` 枚举
- [x] 3.2 移动 `import` → `project import`
- [x] 3.3 移动 `migrate` → `project migrate`
- [x] 3.4 移动 `upgrade-guide` → `project upgrade-guide`
- [x] 3.5 添加旧路径的 deprecated alias
- [x] 3.6 验证

## Phase 4: 移除 archive legacy 别名

- [x] 4.1 移除 `Archive` 结构体中的 legacy 参数
- [x] 4.2 将 `command` 改为必选（不再是 `Option`）
- [x] 4.3 更新 help 文档，移除 legacy 说明
- [x] 4.4 验证

## Phase 5: 简化 `show` 命令选项

- [x] 5.1 设计组合选项方案
- [x] 5.2 实现选项解析器
- [x] 5.3 保留旧选项作为 deprecated alias
- [x] 5.4 验证新旧选项都能工作

## Phase 6: 更新 skills 并移除 deprecated

- [x] 6.1 更新 skills 文件使用新命令空间
- [x] 6.2 移除 `import` 顶层命令
- [x] 6.3 移除 `migrate` 顶层命令
- [x] 6.4 移除 `upgrade-guide` 顶层命令
- [x] 6.5 验证

## Validation

- [x] `cargo fmt -- --check` 通过
- [x] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [x] `cargo test --all` 通过
