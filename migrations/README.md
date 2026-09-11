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
