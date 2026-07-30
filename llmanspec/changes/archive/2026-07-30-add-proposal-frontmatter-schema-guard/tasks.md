# Tasks: add-proposal-frontmatter-schema-guard

## 测试边界（seam）

- 被测边界 = `check_proposal_frontmatter`（`src/sdd/spec/validation.rs`）：传入含未知字段的 frontmatter → 断言返回 `ValidationIssue { level: Error }` 且 message 含合法字段集。
- 复用已有单元测试模式（见 `validation.rs` 中 `proposal_frontmatter_*` 测试）。MUST NOT 另造 CLI 子进程边界（validate 调用链已有集成测试）。
- archived 免检的边界 = 校验入口判断 `changes/archive/` 路径时跳过 frontmatter 未知字段检测。

## Tasks（垂直切片）

- [x] T1: 定义合法字段集 + 未知字段检测（blocked-by: 无）
  - 在 `check_proposal_frontmatter` 中，遍历 frontmatter 的顶层键，凡不属于 `{depends_on, blocks, branch, base_sha, baseSha, checkpointed, checkpoint_sha, checkpointSha}` 的键，push 一条 ERROR issue。
  - ERROR message 含未知字段名 + 合法字段集提示。
  - 单元测试：`proposal_frontmatter_unknown_field_status_reports_error` / `proposal_frontmatter_unknown_field_title_reports_error` / `proposal_frontmatter_allowed_fields_no_unknown_error`（含 camelCase 别名）。

- [x] T2: archived 免检（blocked-by: T1）
  - **由现有架构天然满足**：`list_changes`（`src/sdd/shared/discovery.rs`）枚举 active changes 时显式跳过 `archive` 目录，故 `check_proposal_frontmatter` 永不被 archived proposal 调用。详见 design.md D2。无需额外代码或测试。

- [x] T3: i18n 错误消息（blocked-by: T1）
  - 新增 i18n key `sdd.validate.proposal_frontmatter_unknown_field`（占位 `%{field}` / `%{allowed}`），写入单文件 `locales/app.yml`（本仓 i18n 为扁平单文件，无 en/zh-Hans 分目录）。

- [x] T4: 修正 AGENTS.md「可选 status」措辞 + 同步 skill 模板（blocked-by: 无）
  - 将 `llmanspec/AGENTS.md` 「Change Proposal Frontmatter SSOT」最小 schema 表改为：合法字段集（r124 强制）+ status 已废弃说明 + 生命周期阶段用 `llman sdd status`/`show` 查看推断的 stage（r93 三态）。
  - 同步 draft/propose skill 模板（en + zh-Hans + .agents 渲染版）补 r124 提示。

- [x] T5: 校验全链路 + fmt/clippy（blocked-by: T1, T2, T3）
  - `just check-sdd-templates` / `cargo fmt --check` / `cargo clippy -D warnings` 全绿。
  - `validate` 对 active 含未知字段的 change 报 ERROR（实测 status/title 各一条 ERROR，消息含合法字段集）；archived 由架构免检。
