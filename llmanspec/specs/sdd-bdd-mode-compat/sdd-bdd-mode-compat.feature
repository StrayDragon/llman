# language: zh-CN
# capability: sdd-bdd-mode-compat
# purpose: 规范 sdd 工具链在 BDD-on（Git-native feature-as-spec）与 BDD-off（未启用）两种项目配置下的行为合约与兼容性。当 config.yaml 的 bdd 段存在与否时，validate / change attach|checkpoint|diff / archive / index 等子命令 MUST 表现出符合预期的差异，且 BDD-off 项目不得因 .feature 文件而失败。
# scope: llmanspec/specs/sdd-bdd-mode-compat

功能: sdd-bdd-mode-compat

  @req:r26 @human
  场景: validate 的 check 语义（runner 开关）
    - validate 的 --check/--no-check 行为 MUST 按 bdd 段是否存在切换（此时 bdd 段仅作为 runner 开关，不再影响流程）：含 bdd 段时默认执行 bdd.run_command（对 live 分支树中的真实 .feature），--no-check 跳过；不含 bdd 段时 --check 不执行 runner 且不视为错误（仅输出 INFO）。统一流程下无论 bdd 段有无，validate 都校验 spec.toon 的 requirements 与 Gherkin 解析（若 .feature 存在）。

  @req:r57 @human
  场景: Git-native change binding（统一流程）
    - change MUST 绑定非默认 Git 分支与 immutable base SHA（统一流程，不再按 bdd 段分叉）：`llman sdd change start <id>` 为推荐入口（自动建分支 + clean-tree 门禁 + 绑定，见 sdd-workflow r111）；`llman sdd change attach <id>` 为共存命令（手动绑已有分支，含 --force 重绑）。`checkpoint` MUST 要求干净工作树并跑门禁；`diff` MUST 只读展示/导出 base...HEAD。`llman sdd change new` MUST 能创建 proposal 草稿。`llman sdd change delta` MUST 在任何模式下失败并提示已移除（统一 Git-native，见 sdd-workflow r115）。默认分支上 start/attach/checkpoint/archive MUST 失败。MUST NOT 再提供 `sdd solidify` 子命令。`change checkpoint` 与 `change start` MUST 接受并忽略 `--no-interactive` flag（对齐 change 子命令 flag 矩阵，便于 skill 统一传参）。

  @req:r78 @human
  场景: index rebuild 的 feature embed
    - index rebuild MUST 把 .feature 派生的场景编入 tree.json：约束场景（@human）与验收场景（@executable）均携带其 @req id；无 bdd 段的项目同样编入场景，仅不含 runner 绑定信息。

  @req:r83 @human
  场景: 无 bdd 段时 feature 仍是规格载体
    - validate 在无 bdd 段的项目中 MUST 静默跳过 runner 执行（既不解析可执行绑定也不因格式问题报错），但仍 MUST 做 .feature 的结构校验（头注释、tag 语法学）。单轨格式下无 bdd 段不等于'无 specs'——specs 即 live .feature（在绑定分支编辑）。

  @req:r7 @human
  场景: archive docs rename + ff-merge（统一流程）
    - `llman sdd change archive` 统一行为（不再按 bdd 段分叉）：MUST 先自动 ff-merge feature 分支回分叉点分支，再移动 change 文档到 changes/archive/YYYY-MM-DD-<id>/（详见 sdd-workflow r113）。MUST NOT merge TOON delta（已废除 change/specs 路径）、MUST NOT apply feature_delta。活跃 `*.feature.delta.toon` MUST 作为迁移阻断（ERROR，提示人工清理遗留 delta；partitioned migrate 已移除）。顶层 `sdd archive run` 为兼容别名但 MUST 走统一的 ff-merge 路径。

  @req:r85 @human
  场景: partition-migrate 已移除（零兼容）
    - `llman sdd project migrate --kind partitioned`（及隐藏别名 partition-migrate）MUST 以非零退出拒绝；clap/帮助仅接受 `spec-md2toon`。遗留 change/specs/ 或活跃 *.feature.delta.toon 须人工清理或另开 change（不再提供自动 partitioned 迁移）。错误信息 MUST 提示合法 kind（含 spec-md2toon）。

  @req:r86 @human
  场景: 全局 req_id 唯一性
    - 在 llmanspec/specs 主库中，每个 requirement 的 req_id MUST 在全部 capability 间全局唯一。通用 validate 路径（validate --all、validate <spec>、validate <change> 且会加载主库约束时）MUST 立即检测跨 capability 重复 req_id：默认与 --strict 下 MUST 判为失败并拦截；错误信息 MUST 指出冲突的 req_id 与涉及 capability，并给出可操作修复建议（例如改用 llman sdd spec next-req-id 分配新短 id，或 llman sdd spec resolve-req 查看归属）。非交互场景 MUST NOT 静默放过重复以免积累债务。

  @req:r91 @human
  场景: 批量 validate check 去重
    - BDD-on 且 check mode 开启时：单 spec validate MUST 仍对该 capability 展开 bdd.run_command 占位符后执行一次；validate --all / validate --specs 及任何多 spec 批处理 MUST 按展开后的命令字符串去重，相同展开命令在该次 validate 进程内 MUST 至多实际执行一次，并将通过/失败结果复用到后续命中同一命令的 spec（失败 MUST 仍使相关校验失败）。占位符展开结果不同的命令 MUST 继续按 spec 分别执行。

  @req:r94 @human
  场景: finalize 单 commit 收尾（统一流程）
    - `llman sdd change finalize <id>` MUST 在单进程内执行（统一流程，不再仅限 BDD-on）：门禁（已 start/attach、当前分支 == binding.branch、非默认分支、无遗留 *.feature.delta.toon）→ validate 门禁（live strict + change stage，除非 --no-check）→ 写 frontmatter（checkpointed=true、checkpoint_sha=base_sha）→ 自动 ff-merge → docs-only archive rename（详见 sdd-workflow r113）。finalize MUST NOT 检查工作区 clean tree（使实现 diff + frontmatter + ff-merge + archive rename 可由调用方一次 git commit 收尾；ff-merge 失败时降级为提示且不回滚后续 rename）。finalize 写入的 checkpoint_sha MUST 等于 start/attach 时的 base_sha。finalize MUST 在任何写入前退出非零且不改 frontmatter / 不移动文件（gate 失败或 validate 失败时）。finalize MUST 幂等：重试时若 frontmatter 已含 checkpointed=true 且 checkpoint_sha 非空，MUST 跳过写入直接尝试 ff-merge + archive rename。finalize MUST 接受并忽略 --no-interactive。旧路径 checkpoint + archive（含双重 clean-tree 门禁与严格 sha 语义）MUST 保持不变作 fallback。
