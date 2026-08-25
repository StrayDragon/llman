---
depends_on: []
---

## Why

单轨迁移(v0.0.67 `refactor-spec-format-single-track`)把旧多文件 `.feature` 压缩合并时,sdd-bdd-mode-compat 丢了 1 个 `@executable` 验收场景:「BDD-on 时 validate --no-check 跳过 runner」(原文见 `git show v0.0.66:llmanspec/specs/sdd-bdd-mode-compat/validate-check.feature`)。该行为仍由 `@req:r26 @human` 约束承载,但缺少可执行直测覆盖——现有场景只覆盖「默认执行 runner」与 `--no-check` 的 BDD-off 侧面,未断言 BDD-on 下 `--no-check` 真正跳过 runner。

## What Changes

- 在 `llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature` 的现有 `@req:r26` executable 场景之后,恢复一个 executable 验收场景:给定 BDD-on 项目,`llman sdd validate sample --strict --no-check` 退出码为零且 stderr 不含 `BDD check failed`(按当前单轨格式挂 `@req:r26`)。
- 无代码改动、无 `@human` 合约文本改动(`rules_edit_acked` 不需要):`@human` 场景仅追加同 req 的验收场景。

## Capabilities

- `sdd-bdd-mode-compat`(r26 validate 的 check 语义)

## Impact

- 仅 live spec 验收覆盖 +1;BDD harness(`tests/bdd_steps.rs` 既有 step 库)直接驱动,复用 seam `llman sdd validate <spec> --strict --no-check` CLI 子进程,无需新 step。
