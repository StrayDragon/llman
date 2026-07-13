# Design: Harden git base-ref and env injection

## Decision: denylist（而非 allowlist）

- **Allowlist**（仅允许已知 API key 模式）会误伤合法自定义键，破坏现有「组内任意 env」工作流。
- **Denylist**（拒绝加载器/搜索路径类键）保留现有灵活性，同时堵住 QA 指出的高危面。
- 最小 denylist：`LD_PRELOAD`、`LD_LIBRARY_PATH`、`PATH`、`DYLD_*`（前缀匹配），比较时大小写不敏感。

拒绝策略：**失败并停止**（不静默跳过），避免用户以为已注入。

## Decision: base-ref 校验

- 拒绝空串与以 `-` 开头的值。
- 所有将用户可控 ref 放入 git argv 的路径使用 `--` 分隔（或等价隔离）。
- 校验失败直接错误返回，不发起 merge-base/diff。

## Shared helper

- 抽取共享 `is_dangerous_env_key`（或扩展现有 `is_valid_env_key` 管线），Claude / Codex 注入路径共用，避免两套规则漂移。
