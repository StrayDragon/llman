# 升级到 lifecycle v2（v0.0.75 → v0.0.76）

## 升级 prompt（给用户/agent）

你的 llmanspec 项目需要从 v0.0.75 升级到 v0.0.76（无兼容破坏性升级）。
执行以下步骤：

1. **查看干跑报告**（不写任何文件）：
   ```bash
   python3 migrations/v0.0.75-v0.0.76/upgrade_lifecycle_v2.py
   ```
2. **确认报告内容后执行**：
   ```bash
   python3 migrations/v0.0.75-v0.0.76/upgrade_lifecycle_v2.py --apply
   ```
3. **处理人工清单**：输出中的 `manual:` 项（如 `rules_edit_acked: true` 的 change）
   需要人工确定其实际改动的锁定规则后写入 `rules_touched`：
   ```yaml
   rules_touched: [<req-id>, ...]
   ```
4. **验证全绿**：
   ```bash
   llman sdd validate --all --strict --no-check
   ```

## 本版本破坏性变更（v0.0.76）

- 移除 frontmatter 字段：`skip_specs_landing` → 以 `needs_specs_change`（缺省 true）取代；
  `checkpointed` / `checkpoint_sha` / `checkpointSha` 删除；`rules_edit_acked` 删除
  （`rules_touched` 唯一）；`baseSha` 别名删除（只认 `base_sha`）。
- stage 三态 → 四档：`draft`（仅 proposal）→ `designed`（+design）→
  `planned`（+tasks）→ `full`（+绑定）。
- `llman sdd change checkpoint` 已移除，改用 `llman sdd change finalize`
  （收尾自动提交 `archive(sdd): <id>`；`--no-commit` 可跳过）。
- 新 tag `@agent`（必须与 `@human` 同场景）；`validate --yes` / `finalize --yes`
  只确认带 `@agent` 的规则。

## 脚本行为

- 默认 **dry-run**：只打印将要改写的内容与人工处理清单，不写文件。
- `--apply`：执行改写。跳过 `llmanspec/changes/archive/`（历史只读）。
- 幂等：无内容可改时输出 `no-op`。
- 可选 `--root <path>`：指向其他项目根（默认当前目录）。
