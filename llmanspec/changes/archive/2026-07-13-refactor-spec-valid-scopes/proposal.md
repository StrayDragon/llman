# Proposal: refactor-spec-valid-scopes

## Why

几乎所有主 specs 的 `valid_scope` 都是裸 `src/,tests/`。任意一次 `src/` 小改动都会让 `llman sdd validate --all --strict` 把无关 capability 标为 STALE，门禁信噪比极低，也无法指导 agent 找到真正相关的合约。

## What Changes

- 在 `sdd-workflow` 增加合约：`valid_scope` MUST 精确到所属模块，禁止仅用裸 `src/`/`tests/`（伞形合约除外并须声明）。
- 一次性收窄现有主 specs 的 `valid_scope`（`upgrade-guide` 已精确，保持不动）。
- 对无对应 Rust 实现或偏流程的 specs，scope 落到 schema/templates/CI/scripts，并在 design 标注 UNCERTAIN。

## Capabilities

| Capability | Delta |
|------------|--------|
| `sdd-workflow` | `add_requirement` r38 |

Apply 阶段直接改写各 `llmanspec/specs/<id>/spec.toon` 的 `valid_scope`（见 design 映射表）。

## Impact

- 外部产品行为不变；改变的是 staleness 触发面。
- `validate --all --strict` 在无关改动下不再大面积误报。
- 后续若模块搬家，需同步更新对应 `valid_scope`。
