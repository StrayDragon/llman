# Design: unify-change-lifecycle

记录难逆转 + 无上下文会困惑 + 真实权衡三者皆满足的设计决策（ADR，按 r107 标准）。

## ADR-1: worktree 目录命名 — change-id 为默认，hash 为可选配置

**决策**：worktree 目录名默认 = change-id（已通过 `validate_sdd_id` 校验为安全字符集）；
`sdd.worktree_naming: hash` 可切换为确定性 `base32(sha256(change_id))[:8]` 纯字母。

**权衡**：
- change-id 方案：可发现性高（`ls` 即知）、可读性高、零冲突（change-id 全局唯一）。
- hash 方案：防御性（应对极罕见的项目目录名 lint），但牺牲可发现性。
- 否决「随机 hash」：幂等性破（每次随机则删了重建路径变），必须确定性。

**为何难逆转**：worktree 路径一旦被 agent 习惯，改默认会让存量脚本失效。
**缓解**：配置项保留，默认值变更需 major bump。

## ADR-2: worktree 路径放 .git/sdd/ 而非仓库根

**决策**：默认 `<repo>/.git/sdd/worktrees/<dir>/`，可配 `sdd.worktree_root`。

**权衡**：
- .git/sdd/ 零 .gitignore 污染（git 不碰 .git 子目录的 sdd/）。
- 放仓库根需 .gitignore 条目，易忘。
- IDE 默认忽略 .git 内目录：对 agent 友好（agent 不需要 IDE），人想看时由
  `llman sdd status` 打印绝对路径。

**为何难逆转**：路径约定一旦固化在 attach binding 文档里，迁移需清理存量。
**缓解**：绝不把 worktree path 写进 proposal frontmatter（branch 才是稳定锚，跨机器可移植）。

## ADR-3: archive 自动 ff-merge，失败降级而非回滚

**决策**：archive 在 docs rename 后自动 `git merge --ff-only`；失败时不回滚 rename，
只打印 token 友好提示让 agent/用户手动处理。

**权衡**：
- 回滚 rename 会让 change 处于「已 archive 又回来」的混乱态。
- ff-merge 失败的常见原因（非 fast-forward、分叉点移动）需要人工决策（rebase 还是 merge commit）。
- 不回滚 = archive 的文档侧已完成，只剩 Git 合并侧待办，状态清晰。

**为何难逆转**：决定了 archive 的原子性边界（docs 与 git-merge 解耦）。

## ADR-4: 删除 Specified stage，映射到 Designed

**决策**：stage 从 Draft/Specified/Designed/Full 四态简化为 Draft/Designed/Full 三态。

**权衡**：
- Specified（有 specs 无 design）在新统一流程下不存在——specs 只在分支上动，
  进分支前 = Designed，没进 = 没 specs。
- 旧 Specified 映射：有 design/tasks → Designed；否则 → Draft。
- 代码耦合深（validation.rs / status.rs / validate.rs 三处），重写工作量明确。

**为何难逆转**：外部消费者（CI、脚本）若依赖 `stage=specified` 字符串会断。
**缓解**：本 change 是 zero-compat 重构（已声明不保留 legacy），故接受。

## ADR-5: 零兼容自举策略

**决策**：本 change 要实现 `change start`，但该命令此刻不存在。propose 阶段用现有
`change attach`（手动 git 建分支）进入 full；apply 阶段实现 `change start` 后，
后续 change 即可用新命令。

**权衡**：
- 自举是元规范 change 的固有挑战（r48 已覆盖）。
- 不自举（先发命令再改规范）会导致规范与实现短暂不一致。
- 用 attach 过渡 = 复用已验证的 binding 逻辑，风险最低。

## ADR-6: bdd 段从「流程开关」降级为「runner 开关」

**决策**：`config.yaml` 的 `bdd:` 段不再决定走哪套 change 生命周期流程；
仅决定 `validate --check` 是否执行 `bdd.run_command`。

**权衡**：
- 统一流程 = 一套命令、一套心智模型、一套 skill 模板（token 大幅削减）。
- bdd 段语义收窄后，无 bdd 段的项目也能用 change start/attach/archive + ff-merge。
- `.feature` 文件在无 bdd 段时仅作 Gherkin 文档，不进 runner。
