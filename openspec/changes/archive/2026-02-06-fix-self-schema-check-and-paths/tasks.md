## 1. Implementation
- [x] 1.1 `schema check`：生成的 schema 必须对照样例配置校验；优先使用真实 YAML 文件：
   - global：`LLMAN_CONFIG_DIR/config.yaml`（若存在）
   - project：`<repo_root>/.llman/config.yaml`（若存在）
   - llmanspec：`<repo_root>/llmanspec/config.yaml`（若存在）
   - 否则回退到默认实例作为样例
   - 验证：修改真实 YAML 造成 schema 不匹配时，`schema check` 能失败并报告。
   - 验证：真实 YAML 文件存在但无法读取/无法解析（I/O 或非 UTF-8 或 YAML 语法错误）时，`schema check` 必须失败（不得静默回退 defaults）。
- [x] 1.2 root discovery：`schema apply` 定位 project/llmanspec 配置时向上查找（`.git`、`.llman/`、`llmanspec/`），避免在子目录写错路径。
   - 验证：在子目录运行 `schema apply`，目标路径仍指向 repo 根下的配置文件。
- [x] 1.3 header 最小侵入：仅规范化文件顶部的 header 区域，确保顶部恰好一个正确 schema header，且不删除不相关行。
   - 验证：文件顶部存在多个 schema header 行时，处理后只保留顶部一条正确 header；除被规范化的 header 行外，其余内容保持不变。

## 2. Tests
- [x] 2.1 测试：从嵌套子目录运行 `schema apply` 时的目标路径解析正确。
- [x] 2.2 测试：`schema check` 优先使用真实 YAML 文件作为样例（存在时）。
- [x] 2.3（可选）测试：schema header 处理在多 header 行输入下的最小侵入性。
- [x] 2.4 测试：真实 YAML 存在但不可读/不可解析时 `schema check` 必须失败（不得回退 defaults）。

## 3. Validation
- [x] 3.1 `openspec validate fix-self-schema-check-and-paths --strict --no-interactive`
- [x] 3.2 `just test`

## 4. Acceptance
- [x] 4.1 `schema check` 能覆盖真实配置错误（不再仅验证 defaults）。
- [x] 4.2 `schema apply` 在 repo 子目录运行时不会写错路径。
- [x] 4.3 schema header 修改最小化（只修 header，不改写其它内容）。
