## 1. Spec updates
- [x] 1.1 补充 `sdd-workflow`：归档前置校验 + `--force` 绕过规则
- [x] 1.2 补充 `sdd-workflow`：SDD skills SKILL.md frontmatter 规范要求
- [x] 1.3 补充 `sdd-workflow`：SDD skills 不暴露 `--force` 的约束

## 2. Implementation
- [x] 2.1 为 `llman sdd archive` 增加 `--force` 参数（默认隐藏/弱提示）
- [x] 2.2 归档前对涉及 spec 执行严格校验（含 staleness），失败即中止；`--skip-specs` 跳过
- [x] 2.3 失败提示仅指导修复校验问题，不提示 `--force`
- [x] 2.4 调整 SDD skills 模板 frontmatter：`name`/`description` 合规且与目录一致，保留 `llman-template-version`
- [x] 2.5 （可选）增强 `check-sdd-templates`/测试以校验 SKILL.md frontmatter 规则

## 3. Verification
- [x] 3.1 `just test`
- [x] 3.2 `just check-sdd-templates`
