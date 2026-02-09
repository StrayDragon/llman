## 1. Vendor OPSX 模板（复制并调整）

- [x] 1.1 从上游 OpenSpec repo vendor OPSX 模板到 llman：`templates/sdd/en/spec-driven/*.md` 与 `templates/sdd/zh-Hans/spec-driven/*.md`
  - 覆盖动作：`new`、`continue`、`ff`、`apply`、`verify`、`sync`、`archive`、`bulk-archive`
  - 将内容中的 `openspec/` 路径与 `openspec` CLI 调整为 llman SDD（`llmanspec/` + `llman sdd ...`）
- [x] 1.2 补齐缺失的 OPSX 动作模板：`explore`、`onboard`（从 OpenSpec 上游提取并调整），并保持 en/zh-Hans parity
- [x] 1.3 为 vendored 模板补充来源标记（上游路径 + 日期/版本），并保持 `llman-template-version` 一致策略

## 2. 生成 OPSX commands（slash bindings）

- [x] 2.1 扩展 `llman sdd update-skills`：为 Claude Code 生成 `.claude/commands/opsx/*.md`
- [x] 2.2 为 Codex 生成项目级 `.codex/prompts/opsx-*.md`（与 opsx 动作集合一致；不写入 `$CODEX_HOME` / user-global）
- [x] 2.3 保证“仅生成 OPSX commands”：不创建 legacy commands 目录/文件（如 `.claude/commands/openspec/`）
- [x] 2.4 legacy commands 迁移：检测 `.claude/commands/openspec/` 与 `.codex/prompts/openspec-*.md`，在交互模式下二次确认后删除并迁移到 OPSX（`--no-interactive` 下需报错并提示改用交互模式）

## 3. Workflow skills 对齐 OPSX

- [x] 3.1 补齐 workflow skills（与 opsx 动作集合对应）：`llman-sdd-explore`、`llman-sdd-continue`、`llman-sdd-apply`、`llman-sdd-ff`、`llman-sdd-verify`、`llman-sdd-sync`、`llman-sdd-bulk-archive`
- [x] 3.2 更新 `src/sdd/project/templates.rs` 的模板枚举与 embed 列表（新增 skills 与 opsx 模板）
- [x] 3.3 验证 `llman sdd update-skills --no-interactive --all` 生成：
  - `.claude/skills/**` 与 `.claude/commands/opsx/**`
  - `.codex/skills/**` 与 `.codex/prompts/**`

## 4. 文档与默认引导

- [x] 4.1 更新 `templates/sdd/*/agents.md`：移除“不要添加 tool-specific slash commands”的限制，改为推荐 OPSX commands（并列出最小用法）
- [x] 4.2 更新 `templates/sdd/*/skills/shared.md`：增加 opsx 工作流最小示例与常见故障排查

## 5. 测试与验证

- [x] 5.1 为 commands 生成新增单元测试（动作集合完整、路径正确、重复运行幂等）
- [x] 5.2 运行 `just check-sdd-templates`（模板版本 + locale parity）
- [x] 5.3 运行 `just check`（fmt/clippy/test）
- [x] 5.4 人工 spot-check：在测试仓库执行 `llman sdd init` + `update-skills`，确认 Claude/Codex 均可发现 opsx commands 与 skills

## 6. 规范增量收敛

- [x] 6.1 更新本 change 的 spec delta：明确“仅保留 OPSX commands”与 skills/commands 生成行为
- [x] 6.2 再次运行 `openspec validate upgrade-sdd-openspec-workflow --strict --no-interactive`
