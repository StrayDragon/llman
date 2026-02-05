## 1. Implementation
- [ ] 1.1 显式 `files...` 输入：若是目录/非普通文件，输出清晰提示并跳过（不尝试 `read_to_string`）。
  - 验证：传入目录路径时，结果中不出现“文件读取失败”的误导性错误计数。
- [ ] 1.2 doc comment 开关语义：统一为 `true=允许移除`、`false=禁止移除`、`None=默认保留`（与 line/block 一致）。
  - 验证：配置 `docstrings: true` 时 doc comments 进入候选移除；配置缺省时 doc comments 保留。
- [ ] 1.3 glob 预编译：include/exclude pattern 在 run 级别编译一次并复用。
  - 验证：大批量文件扫描时不再重复编译 pattern（可通过计数/benchmark/日志验证）。

## 2. Tests
- [ ] 2.1 单元测试：目录输入被跳过且不会触发文件读取错误路径。
- [ ] 2.2 单元测试：doc comment 开关语义（true/false/None）与 line/block 语义一致。
- [ ] 2.3（可选）性能测试：pattern 编译次数显著降低（不要求严格基准，但需可观察）。

## 3. Acceptance
- [ ] 3.1 目录输入不再产生噪音错误，用户能理解“跳过原因”。
- [ ] 3.2 doc comment 开关行为可预测且与配置字段名一致。
- [ ] 3.3 不改变 tree-sitter 失败时的“安全失败”策略。

## 4. Validation
- [ ] 4.1 `openspec validate fix-tool-clean-comments-cli-and-rules --strict --no-interactive`
- [ ] 4.2 `just test`
