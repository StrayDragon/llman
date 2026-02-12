## 1. CLI 命令与输出

- [x] 1.1 为 `llman x claude-code account` 增加 `env <GROUP>` 子命令（同时覆盖 `llman x cc ...` 别名路径）
- [x] 1.2 实现 env 语句生成：按 key 排序、校验 key、并按平台输出 `export KEY='value'` 或 `$env:KEY='value'`
- [x] 1.3 完善错误处理：无 groups / group 不存在 / key 非法时非零退出且不输出注入语句
- [x] 1.4 在输出顶部增加以 `#` 开头的用法注释，方便直接运行时复制粘贴

## 2. 测试

- [x] 2.1 新增集成测试：构造临时 `LLMAN_CONFIG_DIR` 下的 `claude-code.toml`，验证 `account env` 输出内容与排序/转义
- [x] 2.2 新增单元测试：覆盖 POSIX 与 PowerShell 的 quoting/escaping 逻辑（包括包含单引号、空格等值）

## 3. CLI 体验

- [x] 3.1 为 `account env` 的 help/README 增加可复制的消费示例（bash/zsh: `eval "$(…)"` 或 `source <(… )`；PowerShell: `... | Out-String | Invoke-Expression`）
- [x] 3.2 增加必要的 i18n 文案（invalid key / group not found 等）并确保 stdout 纯净可管道消费
