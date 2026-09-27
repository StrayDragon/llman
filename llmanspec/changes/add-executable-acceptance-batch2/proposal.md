---
depends_on: []
---

## Why

batch1 压降 pending 97→90 后，剩余 90 条裸规则全部处理，使 review pending 归零、specs 完全达到 llman-sdd 0.5"全部规则配对可执行验收"的主流形态。

## What Changes

- 全部 25 个 capability 的剩余裸规则补嵌套可执行验收场景（90 条），按四种驱动方式：
  1. 仓库自检（本仓库真实工作区）：tests-ci、nightly-toolchain、dependency-upgrade、sdd-workflow migrations SOP。
  2. 技能渲染产物内容断言（fixture 项目 init 产物）：sdd-structured-skill-prompts 全部、sdd-workflow 技能治理规则、prompts r82、sdd-template-units、sdd-specs-compaction-guidance r64。
  3. llman 二进制受控执行：tools 四件套、skills 列表、config 总览/skills --json、worktree、frontmatter 守卫、嵌套 change、max-scan-depth、graph、dirty-tree 门禁、archive 自动合并、context 外置化缺席断言。
  4. llman sdd 委托执行：sdd-context index/context、sdd-review 零配置、openspec 缺席、bdd-mode archive、workflow 各生命周期规则。
- 外置化契约重写：cli r8/r9/r10、config-paths r46（context 能力已外置为 llman-context 插件）改为委托现实描述 + 缺席断言；sdd-openspec-interop r30/r63 改为"上游 0.5 未提供该命令面" + 缺席断言。
- tests/bdd：新增 repo.ts（仓库自检）、skills.ts（技能夹具与渲染产物断言）、tools.ts（tool 四件套夹具）。
