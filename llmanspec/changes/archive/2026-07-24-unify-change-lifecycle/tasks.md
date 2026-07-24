## Tasks

切片按依赖排序。每个切片自含「实现 + 单元测试 + bdd 场景验证」的窄路径。
expand-contract 段（切片 7）用于大范围机械清理。

### 切片 1: stage 三态重构（r93 基础设施）

- [x] 1.1 `src/sdd/spec/validation.rs`: 从 `ChangeStage` 枚举移除 `Specified`；重写 `determine_stage` 为三态（Draft/Designed/Full），去掉 `if bdd_on` 分叉，统一用 attach binding 判 Full
- [x] 1.2 `src/sdd/shared/status.rs`: 移除所有 `ChangeStage::Specified` 匹配臂；`specified` 计数字段与 hint 映射到 Designed
- [x] 1.3 `src/sdd/shared/validate.rs`: stage override 解析 `"spec"` 映射到 Designed（向后兼容输入），移除 Specified 校验分支
- [x] 1.4 `src/sdd/shared/list.rs` / `show.rs`: stage 字段输出三态
- [x] 1.5 更新 validation.rs 内 stage 单测：三态全覆盖（draft/designed/full × bdd on/off 统一）
- [x] 1.6 跑 `cargo nextest run -p llman determine_stage stage` 全绿
- [x] 1.7 跑 bdd `attached-full-unified-bdd-on/off` + `unattached-designed-stays-not-full` 场景全绿

### 切片 2: change start 命令（r111）

- [x] 2.1 `src/sdd/change/start.rs`（新文件）：`run_start` — clean-tree 门禁（`working_tree_clean`）→ 非默认分支检查 → 自动建分支（`sdd.branch_prefix` 配置，默认 `sdd/`）→ 写 attach binding（复用 `write_binding`）
- [x] 2.2 `src/sdd/command.rs`: 注册 `change start <id>` 子命令 + `--worktree` flag + `--no-interactive`（接受并忽略）
- [x] 2.3 clean-tree 失败时输出简练 token 友好错误（`dirty tree: N uncommitted files; commit/stash before change start`），MUST NOT 长篇堆栈
- [x] 2.4 单测：clean tree 通过 / dirty tree 拒绝 / 默认分支拒绝 / 已 attach 未 --force 拒绝 / 成功写 binding
- [x] 2.5 `change attach` 保留为共存命令（手动绑已有分支），两者写同一 frontmatter 结构
- [x] 2.6 跑 bdd `change start 接受 --no-interactive` + `change start 在默认分支上拒绝` 场景全绿

### 切片 3: worktree 支持（r116）

- [x] 3.1 `src/sdd/change/start.rs`: `--worktree` 分支用 `git worktree add` 替代 `git switch`；解析 `<repo>/.git/sdd/worktrees/<dir>/` 路径
- [x] 3.2 配置：`config.yaml` 的 `sdd.worktree_root`（绝对路径覆盖）与 `sdd.worktree_naming`（`id` 默认 / `hash` = `base32(sha256(change_id))[:8]`）；`src/sdd/project/config.rs` 加解析
- [x] 3.3 depends_on 守卫：`--worktree` 时若 `depends_on` 指向未完成 change，非零退出提示串行
- [x] 3.4 目标分支已被 worktree checkout 时复用该路径（`git worktree list` 查询）
- [x] 3.5 `llman sdd worktree prune` 子命令：清理无主 worktree（proposal 已删或已 archive）
- [x] 3.6 单测：worktree 创建 / naming id / naming hash 确定性 / depends_on 守卫 / 复用 / prune
- [x] 3.7 start 返回 worktree 绝对路径供 agent `cd`

### 切片 4: archive 自动 ff-merge（r113）

- [x] 4.1 `src/sdd/change/archive.rs`: docs rename 后追加 ff-merge 步骤——按 attach binding 的 base_sha 反推分叉点分支，`git merge --ff-only <feature> <default>`
- [x] 4.2 ff-merge 失败时：MUST NOT 回滚 rename，只打印 token 友好提示（`ff-merge failed: <reason>; run manually: ...`）
- [x] 4.3 `src/sdd/change/finalize.rs`: 内联 archive 步骤同步加 ff-merge（r94 对齐）
- [x] 4.4 单测：ff-merge 成功 / 非 fast-forward 失败降级 / docs rename 不回滚
- [x] 4.5 跑 bdd `archive 全部 completed 成功并自动 ff-merge` + `ff-merge 失败时降级` 场景

### 切片 5: spec scaffold 命令（r114）

- [x] 5.1 `src/sdd/spec/scaffold.rs`（新）：生成 `llmanspec/specs/<cap>/spec.toon` 骨架（kind/name/purpose/valid_scope/requirements/scenarios 表头 + 示例行，用 next-req-id 分配首个 req_id）
- [x] 5.2 BDD-on 时附带 `.feature` 骨架（`# language` 头 + `@req` 示例场景）
- [x] 5.3 `--help` 与错误提示嵌入格式示例（引号规则、@req 链接、Partitioned SSOT 说明）
- [x] 5.4 拒绝覆盖已存在目录（除非 `--force`）；生成的文件能过 `validate --strict`
- [x] 5.5 单测：scaffold 生成的 spec 直接过 strict validate

### 切片 6: change delta 移除 + 统一拒绝（r115 / r57）

- [x] 6.1 删除 `src/sdd/change/delta.rs`（整个文件）
- [x] 6.2 `src/sdd/command.rs`: 移除 `change delta` 子命令注册；改为统一拒绝 stub（任何模式调用 → 非零退出 + `change delta is removed; edit live specs on a feature branch` 提示）
- [x] 6.3 `src/sdd/spec/parser.rs:112-117`: 移除 change specs_dir 解析
- [x] 6.4 `src/sdd/change/git_native.rs:480-536`: 移除 change/specs 扫描警告
- [x] 6.5 跑 bdd `change delta 在任何模式下都被拒绝`（on/off 两场景）全绿

### 切片 7: archive TOON merge 死代码清理（expand-contract）

- [x] 7.1 `src/sdd/change/archive.rs`: 删除 BDD-off TOON delta merge 路径（已被 r113 ff-merge 取代）
- [x] 7.2 `src/sdd/change/archive.rs`: 删除对应单测（test_archive_merge_toon_delta 等）
- [x] 7.3 archive 统一走 ff-merge 路径，不再有 `if bdd_on` 分叉
- [x] 7.4 跑 archive 相关全量测试，确认无回归

### 切片 8: skill 模板统一（中英双语 × 13）

- [x] 8.1 `templates/sdd/{zh-Hans,en}/skills/llman-sdd-propose.md`: 删 BDD-off delta specs 分支；proposal 只记 why+范围；tasks 用 req_id 引用
- [x] 8.2 `llman-sdd-apply.md`: 删 on/off 分叉；统一「在已 start 的分支上实现」
- [x] 8.3 `llman-sdd-verify.md`: Spec 轴只对 toon+feature；删 on/off 双轨
- [x] 8.4 `llman-sdd-archive.md`: 重写为 docs rename + ff-merge；删 TOON merge 与 on/off 分叉
- [x] 8.5 `llman-sdd-explore.md`: 流程图更新三态；去 on/off 分叉描述
- [x] 8.6 删除 `llman-sdd-sync.md`（中英）；从 `OPTIONAL_SKILL_NAMES`/默认 skills 移除引用
- [x] 8.7 `llman-sdd-continue.md` / `llman-sdd-ff.md` / `llman-sdd-new-change.md` / `llman-sdd-validate.md` / `llman-sdd-show.md` / `llman-sdd-onboard.md`: on/off 措辞与流程图同步
- [x] 8.8 `just check-sdd-templates` 通过（版本头 + locale parity）

### 切片 9: 兼容性测试同步

- [x] 9.1 `tests/sdd_bdd_compat_tests.rs`: 13 子命令 smoke 列表更新（删 delta、加 start/scaffold/worktree prune）
- [x] 9.2 on/off 兼容测试重写：流程统一后「兼容」语义 = attach/start/archive 在 on/off 都可用
- [x] 9.3 `tests/bdd_steps.rs`: 若需要新断言模式（如「worktree 存在」），添加 step；优先复用泛化 step
- [x] 9.4 `src/skills/catalog` 中 OPTIONAL_SKILL_NAMES 与默认 skills 列表移除 llman-sdd-sync

### 切片 10: AGENTS.md + 文档

- [x] 10.1 `AGENTS.md`: 重写「BDD-on（Partitioned SSOT）Conventions」段为「统一 Git-native 流程」；流程图替换为 mermaid 三态图
- [x] 10.2 `AGENTS.md`: bdd 段语义说明改为「runner 开关」；删「如何启用/关闭 BDD-on 模式」里的流程差异
- [x] 10.3 `.agents/skills/` 下所有 llman-sdd-* SKILL.md 通过 `llman sdd init --update` 重新生成
- [x] 10.4 全量 `just check` 通过（fmt + clippy + test + sdd template check）

### 依赖标记

- 切片 2 [blocked-by: 1]（start 依赖三态 stage 判 Full）
- 切片 3 [blocked-by: 2]（worktree 复用 start 的 clean-tree 门禁）
- 切片 4 [blocked-by: 2]（ff-merge 读 attach binding）
- 切片 6 [blocked-by: 4]（delta 移除前 archive 路径须先统一）
- 切片 7 [blocked-by: 4, 6]（TOON merge 清理在 ff-merge 与 delta 移除后）
- 切片 8 [blocked-by: 1, 2, 4, 6]（模板措辞依赖命令与流程定型）
- 切片 9 [blocked-by: 8]（测试 smoke 列表与 skill 列表对齐）
- 切片 10 [blocked-by: 8]（文档对齐模板）
