# Tasks: fix-sdd-bdd-on-change-stage

- [ ] 1. `determine_stage`：BDD-on 用 attach（`branch`+`base_sha`）替代 `change/specs/`；BDD-off 回归保持；补单元测试
- [ ] 2. completeness / locales：attached 完整工件不提示 add specs；未 attach 提示 attach
- [ ] 3. `show`/`list`/`status` 与 `list_change_artifacts` 一致（可选暴露 `attached: true`）
- [ ] 4. Skills 模板 apply/verify/continue：对齐 BDD-on；`init --update` 可刷新
- [ ] 5. live specs：`r93` + feature 场景；必要时更新 skills-contract 措辞
- [ ] 6. `cargo test` / BDD；`llman sdd validate`；checkpoint → archive
