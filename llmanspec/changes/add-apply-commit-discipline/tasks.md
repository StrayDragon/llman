# Tasks

Seam（测试边界，复用既有 harness，无新接缝）：模板门禁
（`just check-sdd-templates`）、渲染一致性（`llman sdd init --update`
diff 核查）。

## T1: apply 模板新增 Commit 策略节

- [ ] `templates/sdd/en/skills/llman-sdd-apply.md`：新增「Commit Policy」
  节（要点见 proposal What Changes；措辞按 design D2）。
- [ ] `templates/sdd/zh-Hans/skills/llman-sdd-apply.md`：同步新增
  「Commit 策略」节，语义对等。
- [ ] 两 locale 模板版本头按 parity 门禁要求同步 bump。
- 验证：`just check-sdd-templates` 绿。Seam：模板门禁。

## T2: resync 渲染产物与全门禁

- [ ] `llman sdd init --update` 重新渲染 `.agents/skills/llman-sdd-apply/`，
  diff 确认仅本节变化。
- [ ] 全门禁：`just check-sdd-templates`、
  `llman sdd validate --all --strict --no-interactive --no-check`、
  `llman sdd validate add-apply-commit-discipline`。
- 验证：全绿；全库 grep 确认无遗漏 locale 树。Seam：模板门禁。
  [blocked-by: T1]
