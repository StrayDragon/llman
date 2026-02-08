## 1. Prompt 注入与配置模型

- [ ] 1.1 在 `templates/sdd/en/prompts/` 与 `templates/sdd/zh-Hans/prompts/` 新增基础 prompts（至少 `workflow-guardrails.md`、`spec-validation-hints.md`）
- [ ] 1.2 扩展 `src/sdd/project/config.rs`：新增 `PromptsConfig` 与 `prompts.custom_path`
- [ ] 1.3 刷新 `artifacts/schema/configs/en/llmanspec-config.schema.json`，确保包含 `prompts.custom_path`
- [ ] 1.4 扩展 `src/sdd/project/regions.rs` 支持 `{{prompt: <name>}}` 占位符
- [ ] 1.5 扩展 `src/sdd/project/templates.rs`，实现 prompt 加载优先级：`custom > project > embedded`（含 locale 回退）

## 2. 工作流 skills 模板补齐

- [ ] 2.1 新增 `templates/sdd/en/skills/llman-sdd-explore.md`
- [ ] 2.2 新增 `templates/sdd/en/skills/llman-sdd-continue.md`
- [ ] 2.3 新增 `templates/sdd/en/skills/llman-sdd-apply.md`
- [ ] 2.4 新增 `templates/sdd/en/skills/llman-sdd-ff.md`
- [ ] 2.5 新增 `templates/sdd/en/skills/llman-sdd-verify.md`
- [ ] 2.6 新增 `templates/sdd/en/skills/llman-sdd-sync.md`（V1 人工作业协议，不含自动合并）
- [ ] 2.7 补齐上述 6 个 `zh-Hans` 对应模板并保持版本一致

## 3. Skills 生成链路更新

- [ ] 3.1 更新 `src/sdd/project/templates.rs` 的 `SKILL_FILES` 列表
- [ ] 3.2 更新 `embedded_template()`，嵌入新增 skills 与 prompts 文件
- [ ] 3.3 验证 `llman sdd update-skills --no-interactive --all` 生成完整技能集

## 4. 测试与验证

- [ ] 4.1 为 prompt 注入新增单元测试（命中、缺失报错、locale 回退、优先级覆盖）
- [ ] 4.2 为 config/schema 变更新增测试（带/不带 `prompts.custom_path`）
- [ ] 4.3 运行 `just check-sdd-templates` 确保模板版本和 locale 文件对齐
- [ ] 4.4 运行 `llman sdd update-skills --no-interactive --all` 并人工 spot-check 生成内容

## 5. 文档与范围校准

- [ ] 5.1 更新本 change 的 `proposal.md` 与 `design.md`，明确不包含 AGENTS 注入
- [ ] 5.2 更新本 change 的 spec delta，将 `sync` 定义为人工作业协议（非自动合并器）
