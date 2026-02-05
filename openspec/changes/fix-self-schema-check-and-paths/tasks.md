## 1. Implementation
- [ ] 1.1 `schema check`：生成的 schema 必须对照样例配置校验；优先使用真实 YAML 文件：
   - global：`LLMAN_CONFIG_DIR/config.yaml`（若存在）
   - project：`<repo_root>/.llman/config.yaml`（若存在）
   - llmanspec：`<repo_root>/llmanspec/config.yaml`（若存在）
   - 否则回退到默认实例作为样例
   - 验证：修改真实 YAML 造成 schema 不匹配时，`schema check` 能失败并报告。
- [ ] 1.2 root discovery：`schema apply` 定位 project/llmanspec 配置时向上查找（`.git`、`.llman/`、`llmanspec/`），避免在子目录写错路径。
   - 验证：在子目录运行 `schema apply`，目标路径仍指向 repo 根下的配置文件。
- [ ] 1.3 header 最小侵入：只修复/替换一个有效 header，确保顶部恰好一个正确 header，且不删除不相关行。
   - 验证：文件中存在多个 schema header 行时，处理后仍保留其余内容不变（仅规范化顶部 header）。

## 2. Tests
- [ ] 2.1 测试：从嵌套子目录运行 `schema apply` 时的目标路径解析正确。
- [ ] 2.2 测试：`schema check` 优先使用真实 YAML 文件作为样例（存在时）。
- [ ] 2.3（可选）测试：schema header 处理在多 header 行输入下的最小侵入性。

## 3. Validation
- [ ] 3.1 `openspec validate fix-self-schema-check-and-paths --strict --no-interactive`
- [ ] 3.2 `just test`

## 4. Acceptance
- [ ] 4.1 `schema check` 能覆盖真实配置错误（不再仅验证 defaults）。
- [ ] 4.2 `schema apply` 在 repo 子目录运行时不会写错路径。
- [ ] 4.3 schema header 修改最小化（只修 header，不改写其它内容）。
