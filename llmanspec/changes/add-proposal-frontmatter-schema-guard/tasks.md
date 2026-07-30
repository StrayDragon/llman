# Tasks: add-proposal-frontmatter-schema-guard

## 测试边界（seam）

- 被测边界 = `check_proposal_frontmatter`（`src/sdd/spec/validation.rs`）：传入含未知字段的 frontmatter → 断言返回 `ValidationIssue { level: Error }` 且 message 含合法字段集。
- 复用已有单元测试模式（见 `validation.rs` 中 `proposal_frontmatter_*` 测试）。MUST NOT 另造 CLI 子进程边界（validate 调用链已有集成测试）。
- archived 免检的边界 = 校验入口判断 `changes/archive/` 路径时跳过 frontmatter 未知字段检测。

## Tasks（垂直切片）

- [ ] T1: 定义合法字段集 + 未知字段检测
  - 在 `check_proposal_frontmatter` 中，遍历 frontmatter 的顶层键，凡不属于 `{depends_on, blocks, branch, base_sha, baseSha, checkpointed, checkpoint_sha, checkpointSha}` 的键，push 一条 ERROR issue。
  - ERROR message 含未知字段名 + 合法字段集提示。
  - 单元测试：`unknown_field_status_reports_error` / `unknown_field_title_reports_error` / `known_fields_no_error`（含 camelCase 别名）。
  - [blocked-by: 无]

- [ ] T2: archived 免检
  - 校验入口（遍历 changes 时）对 `changes/archive/` 下的 proposal 跳过未知字段检测（其他校验如 depends_on 仍保留现有行为）。
  - 单元测试：`archived_proposal_with_status_no_unknown_field_error`。
  - [blocked-by: T1]

- [ ] T3: i18n 错误消息
  - 新增 i18n key（如 `sdd.validate.proposal_frontmatter_unknown_field`），含 `{field}` 与 `{allowed}` 占位。
  - 补 `locales/{en,zh-Hans}/*.yml`。
  - [blocked-by: T1]

- [ ] T4: 修正 AGENTS.md「可选 status」措辞
  - 将 `llmanspec/AGENTS.md` 「Change Proposal Frontmatter SSOT」最小 schema 表中 `status` 行改为：status 已废弃；生命周期阶段用 `llman sdd status` / `llman sdd show` 查看推断的 stage（r93 三态）。
  - [blocked-by: 无]

- [ ] T5: 校验全链路 + fmt/clippy
  - `just fmt` / `just lint` / `just check-sdd-templates` 全绿。
  - `validate` 对 active 含未知字段的 change 报 ERROR；archived 不报。
  - [blocked-by: T1, T2, T3]
