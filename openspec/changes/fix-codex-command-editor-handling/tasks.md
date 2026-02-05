## 1. Implementation
- [ ] 1.1 `$VISUAL`/`$EDITOR` 解析：按 quote-aware 规则拆分为 `cmd + args`，并在最后追加目标配置文件路径。
  - 验证：当 `$EDITOR="code --wait"` 时，实际执行 `code --wait <config-path>`。
- [ ] 1.2 fallback：当 env 为空/仅空白时，仍回退到 `vi`。
  - 验证：unset 环境变量时不报错，仍能打开编辑器（在测试中可用 mock/替身验证调用参数）。

## 2. Tests
- [ ] 2.1 单元测试：editor 命令解析（`code --wait`、带引号的路径、空白处理）。

## 3. Acceptance
- [ ] 3.1 支持带参数的 editor，不改变无参数 editor 的行为。
- [ ] 3.2 不引入 shell 执行或命令注入风险。

## 4. Validation
- [ ] 4.1 `openspec validate fix-codex-command-editor-handling --strict --no-interactive`
- [ ] 4.2 `just test`
