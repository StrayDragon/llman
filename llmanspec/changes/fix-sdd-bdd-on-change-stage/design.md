# Design: BDD-on change stage without change/specs

## Decision

Keep **one** `determine_stage` entry point. Branch on `config.bdd.is_some()`:

| Mode | Specs signal | Full |
|---|---|---|
| BDD-off | `changes/<id>/specs/**` has toon and/or `.feature` (existing `has_spec_files`) | proposal + specs + design + tasks |
| BDD-on | **attach binding**: proposal frontmatter has non-empty `branch` **and** `base_sha` | proposal + design + tasks + attached |

Intermediate stages (BDD-on):

- **Draft**：缺 proposal，或仅有 proposal；或有 design/tasks 但 **未** attach
- **Specified**：proposal + attached（尚无 design/tasks）— 少见，保留对称
- **Designed**：proposal + design + attached，无 tasks
- **Full**：proposal + design + tasks + attached

BDD-off unchanged（含「tasks 无 design → ERROR」约束）。

## Completeness INFO

- Draft + BDD-on + has design+tasks + !attached → hint **`next: attach`**（勿写 add specs）
- Draft + BDD-off → 保留现有 `next: add specs/` 类文案
- Full → 无 completeness 噪声

## Skills

apply/verify：以 `show --json` 的 `stage` 为准（修推断后 `full` 即放行）。文档补充一句：BDD-on 无 `change/specs/` 属预期。continue：仅当真正缺工件或未 attach 时要求「长大」。

## Non-goals

- 扫描 `git diff base...HEAD` 是否改了 live specs（易碎；attach 已表示绑定意图）
- 把 `status: full` frontmatter 当作 stage SSOT（仍由磁盘工件推断）
