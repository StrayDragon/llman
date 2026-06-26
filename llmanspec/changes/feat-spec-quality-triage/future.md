# Future Items

## Deferred: validate --health

`llman sdd validate --health` 的坏口味检测（僵尸 req、迷雾 spec、范围膨胀）延期到独立 change 中实现。

### 触发条件
- `llman sdd context` 命令已稳定使用至少一个迭代周期
- 有明确的坏口味检测规则定义和 false positive 率评估
- 已有实验数据说明检测规则的有效性

### 当前假设
- 僵尸 req：grep 关键词匹配 codebase 和 requirement statement，命中数为 0 时标记 suspected-zombie
- 迷雾 spec：requirement 缺少 scenario 或 scenario 覆盖数 < 1
- 范围膨胀：spec 的 valid_scope 覆盖目录但只有 1 个 requirement

### 不做的内容
- 本 change 的 `list --json --meta` 中预留了 `health` 字段（由 `feat-spec-agent-interface` 实现），但填充逻辑不在此实现
