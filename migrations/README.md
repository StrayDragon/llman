# Migrations（破坏性变更升级目录）

**SOP（sdd-workflow r28）**：任何破坏性合约变更（移除/重命名 frontmatter 字段、
命令、tag 或 stage 值域等）MUST 在同仓库提供版本区间目录 `migrations/v<from>-v<to>/`，
内含：

1. `README.md` — 升级 prompt（给用户/agent 的执行指令），顺序固定：
   dry-run 报告 → `--apply` → 人工处理项 → `llman sdd validate --all --strict` 验证；
2. 一次性升级脚本 — MUST 默认 dry-run（`--apply` 才写文件）、MUST 跳过
   `changes/archive/`（历史只读）、MUST 幂等（重复执行 no-op）；
   无法自动处理的项 MUST 打印人工处理清单，MUST NOT 猜测。

发布说明 MUST 指向对应版本区间的 migrations 目录。

历史目录：

| 版本区间 | 破坏性内容 | 升级路径 |
|----------|-----------|---------|
| `v0.0.75-v0.0.76` | 移除 `skip_specs_landing` / `checkpointed` / `checkpoint_sha` / `checkpointSha` / `rules_edit_acked` / `baseSha`；stage 三态→四档；`change checkpoint` 退役 | `python3 migrations/v0.0.75-v0.0.76/upgrade_lifecycle_v2.py`（见该目录 README） |
| `v0.0.77-v0.0.78` | 锁定规则门禁改报告制；移除 frontmatter 字段 `rules_touched` / `agent_acked`；移除 tag `@agent` | `python3 migrations/v0.0.77-v0.0.78/upgrade_lock_gate_report_only.py`（见该目录 README） |
| `v0.0.78-v0.0.79` | 移除内置 sdd 子系统；`llman sdd` 委托外部 `llman-sdd`（llman-sdd v2 独立发行） | `python3 migrations/v0.0.78-v0.0.79/verify_sdd_external_migration.py`（纯体检，无数据变换；见该目录 README） |
