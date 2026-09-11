# Tasks — git-native-v2

Seam 确认：S1 = 既有回归网全绿（`cargo test --features bdd` 全部场景 + `sdd_bdd_compat.rs` + `validate --all --strict`）；S2 = 本 change 新增 @executable 场景（design §5）与 Rust 测试；S3 = 条文编号保留、内容改写（r98/r10/r118/r134/r135），旧行为兼容矩阵（design §2.4/§3）不回归。任务细节以 design.md 对应节为准。

- [x] T1: range-anchor — `git_utils::resolve_default_branch_ref` 本地优先倒转 + 新增 `effective_range_base`；四消费方换锚（lock_gate / specs_landing / branch_diff+commit_count / staleness base_ref）；分叉 INFO 提示（去重）；无 git 回退存储 base_sha（§2.2-2.3） [blocked-by: 无]
- [x] T2: rules-touched — frontmatter `rules_touched` 数组 + r118 字段集补全；锁定门禁按 req-id 粒度豁免（id→hash 映射复用 validate_locked_scenario 的 rule_req_ids）；bool 兼容读取（§3） [blocked-by: T1]
- [x] T3: specs-landing — r10 landing 判定换新锚 + 条文改写；`sdd review` 的 landing 相关提示同步（§2.3） [blocked-by: T1]
- [x] T4: show-gatechecks — gateChecks 七项检查 + JSON 短键 + text 紧凑段；readyToImplement 与 gateChecks 归一（§4）；show 既有键零变更 [blocked-by: T1, T2]
- [x] T5: specs-landing-contracts — 绑定分支上改写 r98/r10/r118/r134/r135 条文（r134 的 r24 悬空引用改 r135）+ 新增 @executable 场景（§5 两个种子）；**在 apply-cycle 的绑定分支上完成** [blocked-by: T2, T4]
- [x] T6: docs-sync — 根 AGENTS.md（Locked rules 行的 rules_edit_acked → rules_touched 口径）、templates units（stage-guard/ethics-governance/validation-hints 中 ack 字样）、skills（apply/propose/ff/apply-cycle）同步；`just check-sdd-templates` 绿 [blocked-by: T5]
- [x] T7: tests-thicken — design §5 Rust 测试五项 + BDD 场景绑定；`rules_edit_acked` 兼容矩阵回归 [blocked-by: T1, T2, T4]
- [x] T8: full-gate — `just check-all` + `just test-bdd` 全绿；grep 审计「base_sha 用于范围计算」清零（§2.4） [blocked-by: T3, T5, T6, T7]
