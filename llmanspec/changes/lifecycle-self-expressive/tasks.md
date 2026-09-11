# Tasks — lifecycle-self-expressive

Seam 确认（已与需求方逐条确认）：S1 = 既有回归网全绿（`cargo test --features bdd` 全场景 + `validate --all --strict`）；S2 = 本 change 新行为（@executable 场景 + Rust 测试，复用既有 CLI 子进程 / BDD step 库，不发明新 harness）；S3 = 升级工具验证（旧字段 fixture → dry-run → `--apply` → 字段转换正确 + validate 全绿）。任务细节以 design.md 对应节为准。无兼容层：旧字段一律移除（出现即 ERROR），迁移由 `migrations/v0.0.75-v0.0.76/` 承担。

- [ ] T1: stage-four-tiers — `Stage` 四档（draft/designed/planned/full）+ `determine_stage` 判定表（design §2.1）+ `show`/`list`/`validate --stage` 值域 + `gateChecks.stage-complete` 动态 hint（§2.2-2.3） [blocked-by: 无]
- [ ] T2: needs-specs-change — frontmatter 字段（缺省 true）+ 字段集移除 `skip_specs_landing` + specs 目录级 add/remove/update 检查 + landing/gateChecks/validate 指引更新（§3） [blocked-by: 无]
- [ ] T3: checkpoint-removal — 命令移除（单行提示）+ `checkpointed`/`checkpoint_sha`/`checkpointSha` 字段与 `ChangeGitBinding` 清理 + archive 门禁解耦 + r137 引用清理（§4） [blocked-by: 无]
- [ ] T4: finalize-auto-commit — 默认 `git commit -m "archive(sdd): <id>"` + `--no-commit` + 幂等改判（已归档检测）+ 失败语义（§5） [blocked-by: T3]
- [ ] T5: rules-ack-flow — 收尾交互 y/n 写回 + 非交互点名指引 + `--yes`（仅 @agent 规则）+ `@agent` 解析/校验 + `agent_acked` 审计字段与 review/diff 呈现（§6） [blocked-by: 无]
- [ ] T6: migrations-tooling — `migrations/README.md`（SOP）+ `migrations/v0.0.75-v0.0.76/`（README 升级 prompt + 一次性 python 脚本，dry-run 默认、`--apply`、跳过 archive）+ S3 测试（§7、§10-S3） [blocked-by: T2, T3, T5]
- [x] T7: specs-landing-contracts — 绑定分支上改写条文（sdd-workflow r93/r111/r1/r124/r137/r42 + 新升级 SOP 条；spec-format r132/r135；sdd-bdd-mode-compat r42 与 checkpoint 场景）+ 新增 @executable 场景（§10-S2 可执行项）；**在 propose/apply 的绑定分支上完成** [blocked-by: 无]
- [ ] T8: docs-sync — 根 AGENTS.md（四档 / needs_specs_change / commit 自由 / Locked rules）+ templates（feature-contract/validation-hints/stage-guard/git-native-flow）+ skills（apply/archive/propose 等同步 --no-commit、checkpoint 移除、分支提交自由）；`just check-sdd-templates` 绿 [blocked-by: T1, T4, T5, T7]
- [ ] T9: full-gate — `just check-all` + `just test-bdd` 全绿；升级脚本对自己仓库 dry-run 无残留旧字段（active changes 清零） [blocked-by: T2, T3, T4, T5, T6, T8]
