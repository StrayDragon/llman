# Design: refactor-spec-valid-scopes

## Approach

按 capability 拥有的目录/文件设置 `valid_scope`，并附带 `llmanspec/specs/<id>/`。
`sdd-workflow` 作为伞形合约保留较宽的 `src/sdd/,templates/sdd/,...`，但不再用裸 `src/`。

## Mapping（apply 使用）

见 `tasks.md` 勾选清单中的目标 scope 字符串；与 explore 调研一致。UNCERTAIN 项采用保守、可验证路径（schema/templates/CI），不强行指向不存在的 runner 代码。

## Non-goals

- 不改 MUST 产品行为（除新增 r38 治理合约）。
- 不在本 change 修 toon/ison 漂移或实现缺失的 sdd-eval runner。
