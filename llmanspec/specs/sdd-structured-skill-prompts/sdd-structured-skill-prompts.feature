# language: zh-CN
# capability: sdd-structured-skill-prompts
# purpose: 规范 SDD 技能模板的结构化提示协议（Context/Goal/Constraints/Workflow/Decision Policy/Output Contract 及治理字段）。
# scope: llmanspec/specs/sdd-structured-skill-prompts

功能: sdd-structured-skill-prompts

  @req:r32 @human
  场景: SDD 技能结构化提示协议含 Partitioned BDD
    - llman SDD 的技能模板 MUST 采用统一结构化提示协议（Context/Goal/Constraints/Workflow/Decision Policy/Output Contract），并通过模板单元注入组装。Constraints MUST 含 context-first 与 triage。当项目 BDD-on 时，propose/archive/apply/verify/explore 技能 MUST 描述 Git-native Partitioned SSOT：toon=约束层、feature=harness 层、在 feature 分支编辑 live 文件并用 change attach/checkpoint、禁止教导 feature_delta 或 solidify 或 toon 投影覆盖 feature。

  @req:r65 @human
  场景: propose 与 archive 技能对齐 Git-native
    - llman-sdd-propose 与 llman-sdd-archive 技能 MUST 声明：在绑定分支编辑 live `<capability>.feature`，attach/checkpoint 后 docs-only archive，再 Git merge；禁止要求 agent 双写可执行 GWT 或运行 solidify。Git merge 的默认叙事 MUST 为本地 `git merge`（ff-only）进默认分支；skill 正文 MUST NOT 默认导向 `git push` 或 Hosting PR（`gh pr create/merge`）——仅当用户或项目明确要求远程审查时才作为可选步骤出现。apply-cycle / toon-contract 单元与模板 MUST 遵守同一默认叙事。

  @req:r96 @human
  场景: Skill 模板按 BDD 模式条件渲染
    - SDD skill 模板 MUST 经 MiniJinja 按项目 bdd_enabled（config 是否含 bdd:）条件渲染：BDD-on 产物的 propose/apply/verify/archive/explore description 与正文 MUST NOT 将 change 内 delta specs 表述为主要规划产物，MUST 以 feature 分支 live `<capability>.feature` 加 attach/finalize 为主路径；BDD-off 与 BDD-on 统一 Git-native 收尾（change start/attach → finalize/archive）；bdd: 仅影响 runner。对 optional skills（continue/ff/validate/new-change/arch-review/wayfinder/research）的「下一步」推荐 MUST 仅在 config.extra_skills 包含对应项时出现，否则 MUST 给出不依赖该 skill 的替代指引。渲染产物 MUST 保留 per-skill 导航 mermaid（flowchart LR，当前阶段★高亮，含一行位置/下一步文字）；权威 Git-native 生命周期 mermaid 图 MUST 仅保留在 llman-sdd-propose 渲染产物与根 AGENTS.md。sdd-commands 等共享单元 SHOULD 按模式裁剪无关命令行。

  @req:r98 @human
  场景: 收尾提示不默认导向 PR/push
    - 统一 Git-native 下：llman-sdd-apply-cycle 技能 MUST 含「本地合回默认分支」步骤（`git switch <default> && git merge --ff-only <feature>`，可选 `git branch -d <feature>`），且其硬约束 MUST 声明「未获用户明确要求时禁止 git push / gh pr create|merge」。`llman sdd change finalize` 成功 stdout MUST 在归档提示后追加一行 next-step（通常指引在默认分支上 commit archive rename；push / hosting PR 为可选）。`llman sdd validate <change>` 失败时 MUST NOT 打印诱导编写 change 内 TOON delta 的 next-steps（如 `Ensure change has deltas in specs/`），MUST 指向 live `llmanspec/specs/**` 与 `change start`/`attach`。

  @req:r99 @human
  场景: 轻量 draft 提案路径与 change id 自动推导
    - 当用户意图为快速起草提案（如说「draft 提案」「draft change」「记一个提案」且未提供 change id）时，llman-sdd-propose 技能 MUST 走轻量路径：MUST NOT 询问用户确认 change id，MUST 从用户描述内容直接生成一个合法且有意义的 change id（MUST 通过 `validate_sdd_id` 的合法性格式；MUST 遵循该仓库 `llmanspec/AGENTS.md` 声明的命名约定，若无则按描述语义合理命名），并仅创建 `llmanspec/changes/<生成的 id>/proposal.md`（draft shell，不强制 tasks/design/specs/attach）。`llman sdd change new` MUST 支持从描述生成 id：提供 `--from <description>`（或等价）时 MUST 由 CLI 生成 id 并在 stdout 打印最终 id 与 proposal 路径；生成冲突既有 change 时 MUST 以非零退出码失败并提示用 `--force` 覆盖或换描述。技能 MUST 告知用户已生成的 id（可应要求修改）；完整 propose（triage + tasks + specs + attach）仅在用户明确要求正式化时启动。

  @req:r117 @human
  场景: 独立 draft 技能默认安装与职责分离
    - llman SDD MUST 提供一个名为 `llman-sdd-draft` 的默认技能（在 `DEFAULT_SKILL_FILES` 中，随 `llman sdd init --update` 默认安装），职责单一化为「仅创建 draft proposal shell（`change new --from`，不强制 tasks/design/specs/attach）」。该技能 MUST NOT 承担 triage 或完整 propose 职责。`llman-sdd-propose` 技能 MUST NOT 内联完整 draft 路径步骤，MUST 以一句指引导向 `llman-sdd-draft`（如「仅记草案用 llman-sdd-draft」）。曾名为 `llman-sdd-new-change` 的可选技能 MUST 被此默认 `llman-sdd-draft` 取代（从 `OPTIONAL_SKILL_FILES` 移除）；已 init 项目里残留的 `extra_skills: [llman-sdd-new-change]` 条目 MUST 在下次 `init --update` 时被静默忽略（不匹配 optional 列表即过滤），旧 `llman-sdd-new-change` 目录 MUST 被 `cleanup_stale_skills` 自动清理——无需显式迁移代码。
  @req:r60 @human
  场景: review 三时点检查点接线
    - llman-sdd-apply、llman-sdd-verify、llman-sdd-archive 技能模板 MUST 各自在固定时点引导运行 llman sdd review：apply 在每个 task 批次门禁通过后、verify 在报告全绿之后与建议 finalize 之前、archive 在逐个归档执行之前（含批量归档逐个前置）。模板 MUST 声明非零退出等价 CRITICAL 发现：停止当前推进并修复后重跑 review，MUST NOT 在 review 非零退出时继续 finalize 或 archive。explore/propose/draft/quick/graph/specs-compact 技能 MUST NOT 注入该检查点。人审检查点时点 MUST 与根 AGENTS.md「Human Review Checkpoint」一致。

  @req:r71 @human
  场景: 校验修复单元按职责注入
    - 校验修复模板单元（feature 头注释、tag 语法、遗留 spec.toon 迁移指引）MUST 仅注入到会编辑或审查 live specs 的技能（propose/apply/verify/archive/specs-compact）；MUST NOT 注入到 draft/quick/graph 等不编辑 live specs 的技能。注入集合的变化 MUST 经 init --update resync 反映到双 locale 渲染产物。

  @req:r139 @human
  场景: skill 不内嵌命令参考，CLI help 即参考
    - 托管 skill MUST NOT 内嵌命令参考（手写或渲染期生成的命令表均 MUST NOT 存在）：命令的唯一参考面是 CLI 自身的 help（`llman sdd <cmd> --help`），skill 正文 MUST 以一行指引指向它。命令存在性以 clap 命令树为唯一事实源（隐藏子命令排除）；每个可见 sdd 子命令 MUST 保持非空 clap doc comment（en 基线），并有单测守卫。渲染产物 MUST NOT 出现 CLI 已不接受的历史语法（零兼容：用户以版本切换获得对应 skills）。

  @req:r140 @human
  场景: propose 非阻塞推导 change id
    - llman-sdd-propose 技能 MUST NOT 以阻塞式提问向用户索要 change id：用户已给出 id 时 MUST 直接采用；未给出时 MUST 按 r99 的推导规则生成合法 kebab-case id，宣布所用 id 与覆盖方式后继续执行，MUST NOT 等待确认。change id 在 Branch binding 前可低成本更换，MUST NOT 因 id 未确认而中断 propose 闭环。

  @executable
  @req:r99
  场景: change new --from 从描述生成合法 id
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change new --from "add user login"
    那么 退出码为零
    那么 stdout 包含 proposal.md
    那么 stdout 包含 derived change id


  @executable
  @req:r99
  场景: change new --from 冲突既有 change 时失败
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change new add-user-login
    而且 在非交互终端运行 llman sdd change new --from "add user login"
    那么 退出码非零
    那么 stderr 包含 --force
