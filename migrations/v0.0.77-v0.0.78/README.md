# 升级到 lock-gate 报告制（v0.0.77 → v0.0.78）

## 升级 prompt（给用户/agent）

你的 llmanspec 项目需要从 v0.0.77 升级到 v0.0.78（无兼容破坏性升级）。
执行以下步骤：

1. **查看干跑报告**（不写任何文件）：
   ```bash
   python3 migrations/v0.0.77-v0.0.78/upgrade_lock_gate_report_only.py
   ```
2. **确认报告内容后执行**：
   ```bash
   python3 migrations/v0.0.77-v0.0.78/upgrade_lock_gate_report_only.py --apply
   ```
3. **验证全绿**：
   ```bash
   llman sdd validate --all --strict --no-check
   ```

## 本版本破坏性变更（v0.0.78）

- **锁定规则门禁改为报告制（S0）**：`@human` 场景的增删改以 **WARNING** 报告，
  MUST NOT 阻断 validate / change finalize / change diff。
  控制点 = git 分支对比 + `llman sdd review` / `change diff` 报告浮现。
- 移除 frontmatter 字段：`rules_touched`、`agent_acked`（出现即 ERROR，脚本剥离）。
- 移除 tag：`@agent`（不再属于保留字汇；`--yes` 锁定确认语义随之删除）。
- `show` gateChecks 不再包含 `lock-gate` 项。
- 保留：hash→req-id 反查报告、编辑计数、无 `@req` 场景的补标提示、
  有效范围（现算 merge-base，回退存储 `base_sha`）语义。

## 脚本行为

- 默认 **dry-run**：只打印将要改写的文件与被剥离的字段，不写文件。
- 只处理 **active** change（`llmanspec/changes/**/proposal.md`，
  `changes/archive/` 一律跳过——历史归档保持只读）。
- 幂等：无可清理内容时输出 `no-op`。
