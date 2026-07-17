# Tasks

- [x] 1. 合约：`sdd-bdd-mode-compat` 增加 r91 + `validate-check.feature` 批量去重场景
- [x] 2. 实现：bulk validate 对 `run_full_mode` 按 expanded command 缓存（单 spec 不变）
- [x] 3. 文档：`config.yaml` 模板注释标明 batch-once vs 占位符过滤
- [x] 4. 测试：单元测试计数 spawn + BDD step/断言（若需）绑定 `@executable` 场景
- [x] 5. 门禁：`llman sdd validate fix-validate-all-check-batch-once --strict --no-interactive --no-check` 与相关 `cargo test` / `just check` 相关子集
