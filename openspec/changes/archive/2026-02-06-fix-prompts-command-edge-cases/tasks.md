## 1. Implementation（最小可行修复）
- [x] 1.1 模板发现一致性：按 app 的“支持扩展名集合”过滤 `list` 的候选项，并让 `gen` 使用相同的定位规则读取模板。
  - 验证：构造包含混合扩展名/备份文件的目录，`list` 不再展示不可读模板；对展示项执行 `gen` 不再报 “rule not found”。
- [x] 1.2 `prompts rm` 增加 `--yes`：非交互模式下若未传 `--yes`，必须报错并退出非零；传 `--yes` 时无需 prompt。
  - 验证：在非交互环境（例如重定向 stdin）运行两种场景，行为与规范一致。
- [x] 1.3 Claude 注入安全：目标 `CLAUDE.md` 存在但读取失败（I/O、非 UTF-8）时必须终止且不写入。
  - 验证：模拟不可读文件，确认写入路径未发生变化（mtime/content 不变）。
- [x] 1.4 Project root 解析：从任意 repo 子目录运行 project-scope，输出路径必须定位到 `<repo_root>`。
  - 验证：在多层子目录下运行，确认目标写入到 `<repo_root>/.codex/prompts/` 或 `<repo_root>/CLAUDE.md` 等 project 目标。
- [x] 1.5 Project root 缺失护栏：当找不到 git root 时，默认必须拒绝写入；仅允许两种显式绕过：
  - 交互模式：提示用户是否 `--force` 继续（以 `cwd` 作为 root）。
  - 非交互模式：必须显式传入 `--force`。
  - 验证：无 git root 且未 force 时不会写入任何文件；force 时按 `cwd` 下的 project 路径写入。

## 2. Tests（覆盖核心风险）
- [x] 2.1 单元测试：模板过滤/定位（混合扩展名、同名不同扩展、备份文件）。
- [x] 2.2 单元/集成测试：`rm` 在非交互下必须要求 `--yes`。
- [x] 2.3 单元测试：Claude 注入读取失败即停止写入（不可读/非 UTF-8）。
- [x] 2.4（可选）测试：从 repo 子目录运行时的 project root 解析。
- [x] 2.5 测试：无 git root 时 project-scope 必须显式 force（交互确认或 `--force` flag）；未 force 必须失败且不写入。

## 3. Acceptance（验收标准）
- [x] 3.1 所有新增/修改的规范场景在实现后都可通过测试或可重复的手工步骤验证。
- [x] 3.2 不引入新的模板目录结构或输出格式变化（除错误行为修正外）。
- [x] 3.3 非交互删除/注入均具备“显式确认/显式失败”的确定性行为。

## 4. Validation
- [x] 4.1 `openspec validate fix-prompts-command-edge-cases --strict --no-interactive`
- [x] 4.2 `just test`
