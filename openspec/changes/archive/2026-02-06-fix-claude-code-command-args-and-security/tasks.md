## 1. Implementation
- [x] 1.1 交互 args：用 quote-aware parser 替换 `split_whitespace()`，采用确定的 shell-like 拆分语义（支持 `"..."`、`'...'` 与反斜杠转义；未闭合引号/非法转义必须报错且不执行）。
  - 验证：交互输入 `--message "hello world" --flag` 能被解析为 3 个参数（`--message` / `hello world` / `--flag`）。
  - 验证：交互输入包含未闭合引号时会提示错误并允许重新输入（不会执行）。
- [x] 1.2 安全 patterns：统一大小写不敏感匹配（对配置 patterns 做规范化，或在匹配层使用 case-insensitive 逻辑），保证内置与自定义一致。
  - 验证：配置 `RM -RF` 能匹配到 `rm -rf` 的 permission 字符串。

## 2. Tests
- [x] 2.1 单元测试：交互 args 解析（引号、空格、转义、未闭合引号报错）。
- [x] 2.2 单元测试：配置 patterns 的大小写不敏感匹配（包含默认 `contains` 分支）。

## 3. Acceptance
- [x] 3.1 交互模式支持带空格参数，不改变非交互透传行为。
- [x] 3.2 危险模式匹配不因大小写导致漏报。

## 4. Validation
- [x] 4.1 `openspec validate fix-claude-code-command-args-and-security --strict --no-interactive`
- [x] 4.2 `just test`
