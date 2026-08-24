# Tasks: fix-migrate-toon-scenarios-conversion

## 垂直切片

### t1: migrate 转写 + 记账
- [ ] `migrate_capability` 把 `toon_doc.scenarios`（feature=true 且 req_id 配对）传入 `merged.scenarios`，dump 渲染 `@executable`
- [ ] 未配对 feature=true 行计入 `dropped_unpaired`（计数不入 scenarios）
- [ ] 报告字符串区分 `converted_from_toon` / `dropped_notes` / `dropped` / `dropped_unpaired`
- [ ] dry-run 输出预估 converted/dropped_notes
- [ ] 单元测试：feature=true 转写、空列跳过、note 行跳过、未配对记账、幂等、产物 parse 通过
- [ ] `tests/bdd_steps.rs` legacy-toon fixture 补 scenarios[]（feature=true + note 行）

### t2: 合约 + 存量恢复
- [ ] `spec-format.feature` r136 @human 描述更新（转写 feature=true、记账 converted/dropped_notes、配对守卫）
- [ ] `spec-format.feature` r136 增加 @executable 场景：migrate 转写 + 记账产物断言（含 converted_from_toon、@executable、spec.toon 移除）
- [ ] `cli.feature` 恢复 2 条 @executable（r112 baseline / prefix-hint，按修复后转写规则）
- [ ] `llman sdd validate --all --strict` 全绿
- [ ] `cargo +nightly test --features bdd`（BDD 可执行场景）通过