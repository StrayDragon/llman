## 实施记录（可复现评估闭环）

日期：2026-02-27

### 本轮范围

- 优先优化 `apply` 入口（任务 4.1）
  - 清理 `Options/<option>` 等占位内容
  - 去除重复/漂移的 guardrails 表达
  - 统一 STOP/澄清/校验口径（补充 `llman sdd validate`）
  - new + legacy 双轨、`en` + `zh-Hans` 双语同步

### 回归门禁（本仓库）

```bash
just check-sdd-templates
cargo +nightly test --test sdd_integration_tests -q
```

### Arena 深评估（本仓库，一键入口）

准备环境变量（示例）：

```bash
export OPENAI_API_KEY=...
export OPENAI_DEFAULT_MODEL=gpt-5.2
# 如需中转/加速：二选一
export OPENAI_BASE_URL=https://<proxy>/v1
# export OPENAI_API_BASE=https://<proxy>/v1
```

生成 baseline/candidate prompts + 跑 gen：

```bash
just sdd-prompts-eval --rounds 1 --seed 60
```

本次运行产物：
- work_dir：`.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/`
- LLMAN_CONFIG_DIR：`.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/config`
- run_id：`run_60`

### 投票 + 报告

```bash
LLMAN_CONFIG_DIR=.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/config cargo run -q -- x arena vote --run run_60
LLMAN_CONFIG_DIR=.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/config cargo run -q -- x arena report --run run_60
```

结果（rounds=1 的 smoke test，仅覆盖 1 场 match）：
- Leaderboard：
  - `candidate@gpt-5.2`：1516.0（W1 L0 T0 G1）
  - `baseline@gpt-5.2`：1484.0（W0 L1 T0 G1）
- 生成报告：
  - `.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/config/arena/runs/run_60/report.md`
  - `.tmp/sdd-prompts-eval/2026-02-27T112317Z_6e34f2b/config/arena/runs/run_60/ratings.json`

备注：
- `--rounds 1` 只用于验证闭环可跑通；实际评估建议使用 `--rounds 10+` 并完成全部投票。

### Arena 深评估（rounds=10，完整投票）

生成 + 投票 + 报告：

```bash
just sdd-prompts-eval --rounds 10 --seed 60
LLMAN_CONFIG_DIR=.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/config cargo run -q -- x arena vote --run run_60
LLMAN_CONFIG_DIR=.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/config cargo run -q -- x arena report --run run_60
```

本次运行产物：
- work_dir：`.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/`
- LLMAN_CONFIG_DIR：`.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/config`
- run_id：`run_60`

结果（rounds=10，10 场 match，已完成投票）：
- Leaderboard：
  - `baseline@gpt-5.2`：1531.8（W5 L3 T2 G10）
  - `candidate@gpt-5.2`：1468.2（W3 L5 T2 G10）
- 生成报告：
  - `.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/config/arena/runs/run_60/report.md`
  - `.tmp/sdd-prompts-eval/2026-02-27T114048Z_6e34f2b/config/arena/runs/run_60/ratings.json`
