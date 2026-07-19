# Tasks: update-skill-bdd-mode-conditioning

> 顺序执行。每步完成后勾选。

## 1. 合约层（live specs + features）

- [x] 1.1 `sdd-workflow/spec.toon`：新增 **r95**（skill `llman_sdd.bdd_mode` 元信息 +
      validate/init --update/update-skills 一致性 ERROR + 修复提示）。
- [x] 1.2 `sdd-workflow`：新增/扩展 `.feature`（建议 `skills-template-and-commands.feature`）
      含 `@executable @req:r95` 场景：错误 bdd_mode → validate 非零 + stderr 含
      `init --update`；刷新后通过。
- [x] 1.3 `sdd-structured-skill-prompts/spec.toon`：新增 **r96**（MiniJinja 按 bdd_enabled
      条件渲染；description/交叉引用准确性；optional skill 引用门控）。
- [x] 1.4 `sdd-structured-skill-prompts`：补充 harness 或不可执行 scenario 指向检查方式
      （`just check-sdd-templates` / 渲染抽检）。
- [x] 1.5 `sdd-context/spec.toon`：新增 **r97**（context 对 stale/missing 自动 rebuild 再检索）。
- [x] 1.6 `sdd-context`：新增 `.feature` 或单元可验证 scenario（`@req:r97`）；优先可执行。
- [x] 1.7 `llman sdd validate sdd-workflow sdd-structured-skill-prompts sdd-context --strict --no-check` 通过。

## 2. 实现：元信息 + 一致性检查

- [ ] 2.1 模板 frontmatter：所有默认/optional skill 模板写入 `metadata.llman_sdd`；
      `update-skills` 渲染时填入正确 `bdd_mode` / `skill_set`。
- [ ] 2.2 实现 `check_installed_skills_bdd_mode`（或等价）并挂到 validate、init --update、
      update-skills。
- [ ] 2.3 单元/集成测试：mismatch ERROR、缺字段 ERROR、匹配 OK、自定义非 `llman-sdd-*` 忽略。
- [ ] 2.4 i18n 错误串含修复命令。

## 3. 实现：模板条件化与描述修复

- [ ] 3.1 propose/apply/verify/archive/explore/quick（及 units `sdd-commands` /
      `structured-protocol`）按 `bdd_enabled` / `extra_skills` 条件化。
- [ ] 3.2 修复：propose 不以 delta 为 BDD-on 主表述；verify 优先 live specs；continue 引用门控；
      apply-cycle BDD-on 提及 finalize（手动 skill 可简短）。
- [ ] 3.3 保留 mermaid；裁剪与模式无关的命令/空 Ethics。
- [ ] 3.4 `just check-sdd-templates` 通过；对本仓执行 `update-skills` 刷新 `.agents/skills`。

## 4. 实现：context 懒刷新

- [ ] 4.1 `context_run_pageindex`：stale/missing（及合理的 corrupted）→ auto rebuild → retrieve。
- [ ] 4.2 测试：stale 不再返回 `index_stale`；rebuild 后仍缺 chat model 时行为不变。
- [ ] 4.3 同步 `sdd-bdd-mode-compat` / `tests/sdd_bdd_compat_tests.rs` 若断言受影响。

## 5. 文档

- [x] 5.1 新增 `docs/sdd/README.md`、`pipeline-bdd-on.md`、`pipeline-bdd-off.md`（含 mermaid）。
- [x] 5.2 README 注明应急方案 vs draft `add-meta-skill-dynamic-prompts`。

## 6. 门禁

- [ ] 6.1 `just fmt` + `just lint`。
- [ ] 6.2 `llman sdd validate --all --strict --no-check`。
- [ ] 6.3 相关测试 / `just test`（或最小子集 + 说明）。
- [ ] 6.4 建议下一步：`llman-sdd-verify` → `change finalize`。
