---
depends_on: []
branch: fix/validate-all-check-batch-once
base_sha: 3aef37d7ba873fc476be79e81380a4a34c84d7b4
checkpointed: true
checkpoint_sha: a454d5697c1a035c83d32090e3aa355c613005e4
---

## Why

`llman sdd validate --all` / `--specs` 在 BDD-on check mode 下对每个 capability 各跑一次 `bdd.run_command`。当 `run_command` 是项目级套件（无 `{feature_*}` 占位符，如 `cargo test --features bdd`）时，会变成 N× 全量 suite，在大型仓库（如 xylitol ~69 caps）上不可用。见 GitHub #16。

## What Changes

- 单 spec `validate <id>`：保持「对该 spec 展开占位符后跑一次」。
- 批量 `validate --all` / `--specs`（及任何多 spec 批处理）：按**展开后的命令字符串**去重；相同命令在本进程内至多执行一次，后续 spec 复用结果（失败仍使该批校验失败）。
- 含 `{feature_dir}` / `{feature_path}` / `{feature_name}` 且展开结果不同的命令：仍按 spec 分别执行。
- 文档 / `config.yaml` 注释：标明无占位符 runner 在批量校验下为 **batch-once**。

## Capabilities

- `sdd-bdd-mode-compat`：新增 r91（批量 check 去重）；扩展 `validate-check.feature`。

## Impact

- BDD-on + 项目级 `run_command`：`validate --all/--specs` 耗时从 O(N×suite) 降为约 O(1×suite)。
- 过滤型 runner（pytest 等带占位符）行为不变。
- 不改 Partitioned SSOT / attach / checkpoint / archive。

## Non-goals

- 不内置测试 runner；不强制 cargo 用户改用 per-feature filter。
