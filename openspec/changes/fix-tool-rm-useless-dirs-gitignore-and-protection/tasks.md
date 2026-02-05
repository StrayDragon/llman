## 1. Implementation
- [ ] 1.1 默认 `.gitignore`：当用户提供扫描目标 `path` 且未显式传 `--gitignore` 时，默认查找 `<target>/.gitignore`；若未提供 `path`，默认仍为 `./.gitignore`。
  - 验证：对非 CWD target 执行时，忽略命中行为与 target 的 `.gitignore` 一致。
- [ ] 1.2 protected 组件保护：扫描与递归 MUST 在遇到 protected 组件时停止进入该子树（不遍历、不删除）。
  - 验证：`some/.git/objects` 不被遍历；`node_modules` 即使被 ignore 且启用 `--prune-ignored` 也不被删除。
- [ ] 1.3（可选）减少二次遍历：避免 useless dir 删除前的重复递归扫描，保持保护语义不变。
  - 验证：在包含大量缓存目录的 fixture 中运行耗时显著降低（允许粗粒度验证）。

## 2. Tests
- [ ] 2.1 测试：默认 gitignore 相对 target 解析（非 CWD target）。
- [ ] 2.2 测试：protected component traversal（遇到 `.git`/`node_modules` 等组件时跳过子树）。

## 3. Acceptance
- [ ] 3.1 默认 `.gitignore` 行为与用户直觉一致（扫描哪个目录就用哪个目录的 `.gitignore`）。
- [ ] 3.2 protected 子树永不删除、永不遍历（符合 spec）。

## 4. Validation
- [ ] 4.1 `openspec validate fix-tool-rm-useless-dirs-gitignore-and-protection --strict --no-interactive`
- [ ] 4.2 `just test`
