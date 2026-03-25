## 1. CLI 透传实现

- [x] 1.1 在 `ClaudeCodeArgs` 上增加 `args: Vec<String>`（`trailing_var_arg`）并启用 `args_conflicts_with_subcommands`
- [x] 1.2 主命令路径将 `--` 后参数按顺序透传给 `claude`（保留安全检查与 env 注入顺序）
- [x] 1.3 校对 `llman x cc --help` / `llman x claude-code --help` 的文案与示例

## 2. 测试覆盖

- [x] 2.1 新增集成测试：`llman x cc -- --version` 能被 clap 捕获并最终传递给 `claude`
- [x] 2.2 新增/调整测试：确保 `llman x cc run ...` 等子命令解析不受影响

## 3. 验证

- [x] 3.1 运行 `just test`（或 `cargo +nightly test --all`）确保通过
