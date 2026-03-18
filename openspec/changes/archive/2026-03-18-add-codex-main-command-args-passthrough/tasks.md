## 1. CLI 行为

- [x] 1.1 为 `llman x codex` 主命令新增 `-- <codex-args...>` 捕获并避免与子命令歧义
- [x] 1.2 在交互选组完成后，将捕获到的 args 透传注入到实际 `codex` 执行参数中

## 2. Tests

- [x] 2.1 增加 CLI 解析单测：`llman x codex -- --help -m o3` 能正确解析并保留透传参数

## 3. Validation

- [x] 3.1 运行相关测试（至少覆盖 `src/x/codex/command.rs` 单测）并确保通过
