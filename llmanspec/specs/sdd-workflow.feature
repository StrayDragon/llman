# language: zh-CN
# capability: sdd-workflow
# purpose: 定义 llman SDD 规范驱动开发工作流及其 `llmanspec/` 行为合约。
# scope: llmanspec/specs/sdd-workflow.feature

功能: sdd-workflow

  @req:r1
  规则: Specs landing 与 apply-ready 门禁
    Git-native 流水线 MUST 区分 Branch binding 与 Specs landing：live `llmanspec/specs/**` 的编辑与提交 MUST 仅发生在 change 已绑定的非默认 feature 分支上；默认分支 MUST NOT 因过 change start 干净树门禁而接收未实现合约。Specs landing MUST 判定为 `git diff --name-only <effective-range-base>...<binding.branch> -- llmanspec/specs` 触及 `llmanspec/specs/` 下任意文件（add/remove/update 任一；看 binding 分支 tip，非当前 HEAD；effective-range-base = 现算 `git merge-base <本地默认分支> HEAD`，见 r111 审计说明——MUST NOT 用存储 base_sha 计算范围）。proposal frontmatter 正向字段 `needs_specs_change`（缺省 true）控制该检查：true（缺省）→ diff 无 specs 改动 MUST 报错并给指引（改 specs / 声明 false / 走 quick 路径）；false → 跳过该项检查。`skip_specs_landing` MUST 被移除（无兼容读取，出现即未知字段 ERROR）。`llman sdd show <id> --type change --json` MUST 暴露 `specsLanded`、`needsSpecsChange`、`readyToImplement` 与聚合门禁数组 `gateChecks`。readyToImplement MUST 为 stage=Full 且 gateChecks 全部通过（其中 specs-landed 项通过 = needs_specs_change=false 或 diff 触及 specs），否则 false。`llman sdd validate` 对 Full 且未 ready MUST 报 WARNING，消息 MUST 含 skill 引导（如 llman-sdd-propose / llman-sdd-apply）并 MUST NOT 建议对已 attach 的 change 再跑 change start。默认分支上对 `llmanspec/specs` 的未提交脏改动 MUST 以 WARNING 提示改到绑定分支。丢失绑定分支上的 specs 改动时 MUST 走恢复（checkout/重建分支 + 必要时 attach --force），MUST NOT 与 start 概念混淆。

    场景: attached-with-needs-specs-change-false-is-ready
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r1-skip 含 proposal design tasks 且 attach 状态为 "skip"
      当 在非交互终端运行 llman sdd show r1-skip --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "full"
      而且 stdout 的 JSON 键 needsSpecsChange 为 "false"
      而且 stdout 的 JSON 键 readyToImplement 为 "true"
      而且 stdout 的 JSON 键 specsLanded 为 "false"

    场景: needs-specs-change-true-passes-with-specs-diff
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r1-landed 绑定于已提交 specs 改动的分支
      当 在非交互终端运行 llman sdd show r1-landed --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 specsLanded 为 "true"
      而且 stdout 的 JSON 键 needsSpecsChange 为 "true"

    场景: show-gatechecks-compact
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 gc-dev 含 proposal design tasks 且 attach 状态为 "yes"
      当 在非交互终端运行 llman sdd show gc-dev --type change
      那么 退出码为零
      那么 stdout 包含 gateChecks
      那么 stdout 包含 specs-landed
  @req:r39
  规则: SDD list JSON 含 morphology
    llman sdd list --specs --json output MUST include purpose validScope health staleness 以及 morphology 对象。morphology MUST 含原生五键 ruleCount ruleEnforcedCount rulePendingCount acceptanceCount featureScenarioCount（数值，键名口径见 spec-format r134）。purpose 仍来自 spec；health 与 staleness 在质量检测未实现前可为 null。harnessBoundCount 与 harnessUnboundCount 及 bdd.bindings 绑定口径 MUST 退役（不再输出、不再读取）。
  @req:r47
  规则: Context JSON semantic protocol
    llman sdd context output JSON MUST follow: direct array contains z-score greater than 0.60 specs agent MUST read full spec. related array contains z-score greater than -0.20 and at most 0.60 specs agent MAY read on demand. remaining specs are irrelevant. summary MUST include readRecommended and staleWarnings fields.
  @req:r48
  规则: 变更规模分类与路径选择（Triage）
    SDD 工作流 MUST 在提案阶段前引入变更规模分类步骤，帮助 agent 选择合适的工作路径。分类规则 MUST 包含：- 行为合约变更：修改 MUST/SHALL 定义的外部可观测行为 → 走完整 SDD 流程（proposal + tasks；design 可选；进 feature 分支编辑 live specs；archive 合并收口）- 实现变更：不改变外部行为只改变内部实现 → 走快速路径（直接改代码，无需 change 目录）- 治理/工具变更：修改 CI/工具配置 → 仅创建 proposal.md 记录 why- 元规范变更：修改 SDD 规范/模板/流程本身 → 走完整 SDD 流程（自举）当变更性质不明确时 agent MUST 升级为完整 SDD 流程而非猜测。统一 Git-native 流程下不再有 change/specs/ delta 路径。
  @req:r61
  规则: 统一 Git-native 流水线
    流水线 propose/apply/verify/archive MUST 统一为 Git-native 单轨流程，不再区分 BDD-on / BDD-off 的命令分叉：Designed 阶段仅维护 changes/<id>/ 规划壳；change start（或 attach）完成 Branch binding 后，才在绑定分支编辑 live specs（每个 capability 唯一的 <capability>.feature：原生 功能→规则→场景 分层，规则块挂 @req 句柄、嵌套场景为验收示例、顶层场景为功能级示例，见 spec-format）形成 Specs landing；archive 在 docs rename 后自动合并回分叉点分支（目标与方式解析见 r113，squash 缺省）才是 specs 合入默认分支的正常窗口。Skills 与 validate 阶段感知 MUST 与此统一流程及 r1 Specs landing 门禁一致。MUST NOT 提供 change delta / solidify / feature_delta / change/specs/ delta 路径（已废除，零兼容）。
  @req:r87
  规则: spec next-req-id 分配器
    llman sdd spec next-req-id MUST 扫描 llmanspec/specs 主库已占用的全部 req_id，并向 stdout 打印一个当前未使用的短候选 id。默认形态 MUST 为 rN（N 为正整数且未占用）；--json 时 MUST 输出可解析 JSON 且含 reqId 字段。MUST NOT 要求把 capability 编入候选 id。

    场景: next-req-id-json
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd spec next-req-id --json
      那么 退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId
  @req:r88
  规则: add-req 拒绝全局已占用 req_id
    llman sdd spec add-req 在调用方提供的 req_id 已在 llmanspec/specs 主库任一 capability 中占用时，MUST 以非零退出码失败并在 stderr 说明冲突 capability，MUST NOT 写入会破坏全局唯一性的 requirement 行。守卫按全局注册表（@req:<id> 句柄）占用口径生效。

    场景: add-req-rejects-global-collision
      假如 已初始化含已占用全局 req_id 的 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd spec add-req sample r1 --title t --statement "MUST keep unique"
      那么 退出码非零且 stderr 包含 already in use
  @req:r89
  规则: spec resolve-req 解析归属
    llman sdd spec resolve-req <req_id> MUST 在主库中解析该短别名的归属并打印 reqId、capability、title 等归属行。找不到时 MUST 非零退出。该命令 MUST 作为 agent 获取 req 关联数据的一等入口，替代把 capability 编码进 req_id。

    场景: resolve-req-reports-ownership
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd spec resolve-req r1
      那么 退出码为零
      那么 stdout 包含 reqId
      那么 stdout 包含 capability
  @req:r90
  规则: init-update 清理废弃 llman-sdd-*
    `llman sdd init --update`（及等价 skills 刷新）MUST 以默认 workflow skills + `config.yaml` 的 `extra_skills` 为候选集；先删除 `.agents/skills/` 下不在候选集中的 `llman-sdd-*` 目录，再写入/更新候选。MUST NOT 删除无 `llman-sdd-` 前缀的自定义 skill。清理时 stderr MUST 含 `Cleaned up stale skill`。

    场景: init --update 清理废弃 llman-sdd 技能
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      假如 项目中存在技能目录 llman-sdd-solidify
      假如 项目中存在技能目录 my-custom-skill
      当 在非交互终端运行 llman sdd init --update
      那么 退出码为零
      那么 stdout 包含 removed: llman-sdd-solidify
      那么 相对路径 .agents/skills/llman-sdd-solidify 不存在
      那么 相对路径 .agents/skills/my-custom-skill 存在
      那么 相对路径 .agents/skills/llman-sdd-explore 存在

    场景: extra_skills 扩展候选不被清理
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      假如 项目 extra_skills 包含 llman-sdd-continue
      当 在非交互终端运行 llman sdd init --update
      那么 退出码为零
      那么 相对路径 .agents/skills/llman-sdd-continue 存在
  @req:r92
  规则: archive freeze --list 枚举冻结条目
    `llman sdd archive freeze --list` MUST 枚举冷备份归档 freezed_changes.7z.archived 内已冻结的 change 目录（顶层 YYYY-MM-DD-id 命名）。MUST 为只读：不执行任何冻结、删除或写入。当归档文件不存在时 MUST 打印明确提示并以退出码 0 结束。可执行行为见 archive-freeze-and-gates feature。

    场景: freeze-list-无归档文件时提示并退出零
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd archive freeze --list
      那么 退出码为零
      而且 stdout 包含 contains no archived changes
  @req:r93
  规则: 统一四档 stage（Draft/Designed/Planned/Full）
    determine_stage（及 show/list 同源）MUST 统一采用四档，每档名与完成物对齐：- Draft：仅 proposal.md（或 tasks 有而 design 无——另有 ERROR 禁令）。- Designed：proposal + design.md 存在即达（**不需要 tasks**）。- Planned：proposal + design + tasks 齐全，但尚未绑定。 - Full：Planned + frontmatter 含非空 branch 与 base_sha（已 change start / attach）。绑定不改变文件档位（attached 但缺 tasks 时仍为 Designed，attached 单独暴露）。archive 的只读旧数据不迁移。readyToImplement MUST 遵循 r1，MUST NOT 仅因 stage=Full 即为 true。MUST NOT 再读取 changes/<id>/specs/ 作为规格信号（该目录已废除）。Skills apply/verify MUST 与此四档及 r1 语义一致。「有 tasks 无 design」MUST 报 ERROR（依赖链 proposal ← design ← tasks）。show MUST 显示当前档位与距下一档缺什么。

    场景: attached-full-unified-bdd-on
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r93-attached 含 proposal design tasks 且 attach 状态为 "yes"
      当 在非交互终端运行 llman sdd show r93-attached --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "full"
      而且 stdout 的 JSON 键 attached 为 "true"
      而且 stdout 的 JSON 键 readyToImplement 为 "false"
      而且 stdout 的 JSON 键 specsLanded 为 "false"

    场景: unattached-three-artifacts-is-planned
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r93-bare 含 proposal design tasks 且 attach 状态为 "no"
      当 在非交互终端运行 llman sdd show r93-bare --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "planned"
      而且 stdout 的 JSON 键 readyToImplement 为 "false"

    场景: draft-stage-proposal-only
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r93-only 仅含 proposal
      当 在非交互终端运行 llman sdd show r93-only --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "draft"
      而且 stdout 的 JSON 键 readyToImplement 为 "false"

    场景: designed-stage-design-without-tasks
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 r93-design 含 proposal 与 design 不含 tasks
      当 在非交互终端运行 llman sdd show r93-design --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "designed"
      而且 stdout 的 JSON 键 readyToImplement 为 "false"

    场景: attached-full-unified-bdd-off
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      而且 变更 r93-off 含 proposal design tasks 且 attach 状态为 "yes"
      当 在非交互终端运行 llman sdd show r93-off --type change --output json
      那么 退出码为零
      而且 stdout 为合法 JSON
      而且 stdout 的 JSON 键 stage 为 "full"
      而且 stdout 的 JSON 键 readyToImplement 为 "false"
      而且 stdout 的 JSON 键 specsLanded 为 "false"
  @req:r100
  规则: explore grilling 深对齐分支
    llman-sdd-explore 技能 MUST 支持可选 grilling 分支：仅当用户显式触发（如说『深挖』『grill』『逐个问』『彻底理清』）时进入。该分支 MUST 一次只问一个问题并附推荐答案；MUST 优先通过读取 <capability>.feature/代码/运行命令自行查证事实而非询问用户，仅把决策性问题交由用户；MUST 将已解决的决策回写到该 change 的 proposal.md（在绑定分支上写 live 文件）。完成判据 MUST 为决策树每一分支均已解决或显式 defer。默认 explore 行为（问 1-3 个问题）MUST 不变。
  @req:r101
  规则: propose seam 前置确认与垂直切片 tasks
    llman-sdd-propose 技能在写 tasks.md 前 MUST 先列出将测试的 seam 并与用户确认：seam 定义为 *.feature 的 GWT 步骤所驱动的公共边界（CLI 子进程或 public interface），MUST NOT 另行发明脱离已有 harness 的 seam。tasks.md MUST 按 tracer-bullet 垂直切片组织（每个 task 切穿 schema→API→UI→tests 的完整窄路径且可独立验证），并 MUST 支持 [blocked-by: <task-id>] 依赖标记。wide refactor 例外 MUST 按 expand-contract 顺序排列。propose 顺序 MUST 为：充实 proposal/design/tasks → change start 或 attach → 再在绑定分支编辑 live specs（MUST NOT 先改 live specs 再 start；MUST NOT 为过干净树门禁把 live specs commit 到默认分支）。
  @req:r102
  规则: apply diagnose 紧反馈升级路径
    llman-sdd-apply 技能在自修复循环失败时 MUST 升级到 diagnose 子流程：MUST 先构造一个 red-capable 命令（紧、确定、快、agent 可运行，且能在此 bug 上转红），然后最小化 repro，再生成 3-5 个排序假设并单变量验证。MUST NOT 在无 red-capable 命令前进入假设阶段。判定为非 hard bug 时 MUST 保持现有最小修复逻辑不变。
  @req:r103
  规则: verify 双轴审查
    llman-sdd-verify 技能的审查 MUST 分两轴分离呈现且互不污染：Spec 轴（实现是否满足 *.feature 中规则块的需求描述（含 MUST/SHALL 语义的自由文本）与嵌套场景的 GWT 验收，含缺失/部分/scope creep/错误实现）与 Standards 轴（代码是否符合 AGENTS.md coding style 加 Fowler smell baseline：Mysterious Name/Duplicated Code/Feature Envy/Data Clumps/Primitive Obsession/Repeated Switches/Shotgun Surgery/Divergent Change/Speculative Generality/Message Chains/Middle Man/Refused Bequest）。Standards 轴的权威优先级 MUST 为 AGENTS.md 文档标准高于 smell baseline，且 MUST 跳过 tooling 已强制项。smell 标记 MUST 为判断性启发而非硬违规。
  @req:r104
  规则: arch-review 独立 skill
    系统 MUST 提供 llman-sdd-arch-review 技能（model-invoked，经 config.extra_skills 可选启用）：扫描 codebase 的 shallow module 产出 deepening 候选，每候选含 files/problem/solution/benefits/recommendation strength。MUST 复用 codebase-design 词汇（module/interface/depth/seam/adapter/leverage/locality）。触发词 MUST 含『架构审查』『deepening』『shallow module』。
  @req:r105
  规则: wayfinder 独立 skill
    系统 MUST 提供 llman-sdd-wayfinder 技能（user-invoked，disable-model-invocation 为 true）：将大型雾状工作规划为 decision ticket map，每个 ticket 解决一个决策而非交付一个切片。MUST 与 llman 的 change 与 graph 命令结合表达 frontier/blocking。MUST 支持 fog-of-war（Not yet specified 段）与 out-of-scope 区分。
  @req:r106
  规则: research 独立 skill
    系统 MUST 提供 llman-sdd-research 技能（model-invoked，经 config.extra_skills 可选启用）：以后台 agent 委托外部文献调研，针对 primary sources（官方文档/源码/第一方 API）而非二手转述，产出 cited markdown 回写到该 change proposal 的 Further Notes 段。
  @req:r107
  规则: 领域语言治理回写 .feature
    llman-sdd-explore 的 grilling 分支与领域语言治理 MUST 在遇到术语冲突或模糊词时挑战并 sharpening：解决后 MUST 更新对应 <capability>.feature 中的规则块标题与描述（在绑定分支编辑 live 文件）。MUST NOT 另建 CONTEXT.md glossary 作为第二权威。ADR 记录 MUST 仅当『难逆转 + 无上下文会困惑 + 真实权衡』三者皆满足时建议，记入 design.md。
  @req:r108
  规则: AGENTS.md 增强能力路由
    AGENTS.md 的 SDD 段 MUST 含『可选增强能力』小节，索引 r100-r106 增强能力的触发条件（grilling/diagnose/双轴 verify/arch-review/wayfinder/research）。MUST 固化 seam 与 depth 等借自 codebase-design 的词汇定义，避免与 llman 已有词汇双轨。MUST NOT 另建 ask-matt 式 router skill。
  @req:r109
  规则: config 命令总览
    llman sdd config（无子命令时）MUST 打印当前项目 config 摘要：schema、locale、extra_skills 启用数量与列表、bdd 是否启用（on/off）、archive 配置状态。作为只读快速查看入口，MUST NOT 修改 config.yaml。
  @req:r110
  规则: config skills 非交互管理
    llman sdd config skills MUST 为非交互命令：打印当前 extra_skills 启用列表与全部候选的 available 列表（不修改文件）；--json 时 MUST 输出含 enabled 与 available 数组的 JSON。交互式多选 MUST 移除。写回 extra_skills 由用户直接编辑 config.yaml 后运行 llman sdd init --update 落地 skill 文件。
  @req:r111
  规则: change start 自动进分支与干净门禁
    llman sdd change start <id> MUST 在单进程内完成 planning 档（Draft/Designed/Planned 任一）→ Full 的 Git-native 切换：先校验工作区 MUST 干净（git status --porcelain 为空），不干净时 MUST 以非零退出失败并输出简练的 token 友好错误（如 'dirty tree: N uncommitted files; commit/stash before change start'），MUST NOT 长篇堆栈或建议清单；校验通过后 MUST 要求当前在默认分支上（已在非默认分支时 MUST 提示改用 change attach，或先切回默认分支），然后自动创建 feature 分支（命名规则 sdd/<change-id> 或可配 sdd.branch_prefix）、写 attach binding（branch + base_branch + base_sha；base_branch = fork 基准分支——start 语境恒为本地默认分支，`change attach` 缺省写默认分支且 MUST 支持 `--base <branch>` 显式覆盖以记录 stacked fork 源，`--force` 重绑时随当前状态重算；base_sha = 绑定时与默认分支的 merge-base，**仅作审计/溯源记录**；一切 diff 范围语义 MUST 使用现算 `git merge-base <本地默认分支> HEAD`，base_branch MUST NOT 参与 diff 范围计算（锁定哈希门禁已退役，见 spec-format r135），仅用于 r113 的合并目标解析，见 r1/r130）到 proposal frontmatter，并打印新建分支名与 base SHA。change start MUST NOT 把默认分支写为 binding.branch。已 attach 且未带 --force 时 MUST 报错提示当前绑定分支。change attach（手动绑已有分支）MUST 作为 change start 的共存命令保留：当用户已手动 git switch -c 到分支、或需要绑定非 sdd/ 前缀分支时使用；两者写入同一 frontmatter binding 结构。start 仅完成 Branch binding，MUST NOT 表示 Specs landing 已完成或 readyToImplement 已为 true。
  @req:r116
  规则: change start worktree 并行与依赖守卫
    llman sdd change start <id> --worktree MUST 用 git worktree add（而非 git switch）创建独立工作树，使多个 change 可并行 checkout。worktree 路径默认为 <repo>/.git/sdd/worktrees/<dir>/，<dir> 默认等于 change-id（已为安全字符集）；可通过 config 的 sdd.worktree_root（绝对路径）与 sdd.worktree_naming（id|hash，hash = 确定性 base32(sha256(change_id))[:8] 纯字母）配置。worktree 路径 MUST NOT 写入 proposal frontmatter（branch 才是稳定锚，路径仅本机有效）。若 change 的 depends_on 指向未完成的 change，change start --worktree MUST 以非零退出失败并提示串行处理（或先完成依赖）；当 depends_on 为空或指向已完成 change 时允许并行。当目标分支已被某 worktree checkout 时，change start MUST 复用该 worktree 路径而非报错。系统 MUST 提供 llman sdd worktree prune 子命令清理无主 worktree（对应 proposal 已删除或已 archive 的）。
  @req:r113
  规则: archive 自动合并回分叉点分支（目标/方式可解析，squash 缺省）
    llman sdd change archive <id>（及 finalize 内联的 archive 步骤）MUST 先对 attach binding 的 feature 分支执行自动合并到合并目标分支，再将 change 文档 docs rename 到 changes/archive/YYYY-MM-DD-<id>/（未提交的 rename 若先于合并会被 feature tip 还原，故顺序为 merge→rename）。合并目标 MUST 按「显式 `--into <branch>` > binding 记录的 base_branch（fork 基准分支；旧绑定缺键时回退）> 本地默认分支」解析；合并方式 MUST 按「显式 `--method <squash|ff>` > config `sdd.merge_method`（缺省 squash）」解析。squash 方式（缺省）MUST 使目标分支恰好新增一个收口 commit：squash 暂存 feature 全部 diff、docs rename 与收尾提交合而为一（message 沿用 `archive(sdd): <change-id>`）；ff 方式 MUST 保留原语义（`git merge --ff-only` 原样带入 feature commits，再由调用方一次 commit 收尾 rename）。成功则打印合并结果并留在目标分支。合并无法执行（目标分支被其他 worktree 持有、非 fast-forward、checkout 失败等）时 MUST NOT 静默降级：MUST 输出显式 WARNING 与可执行的手动合并命令指引，MUST NOT 跳过后续 docs rename，且 MUST NOT 因合并失败回滚 docs rename（worktree 拓扑守卫细则见 r142）。统一流程下不再有 TOON delta 合并路径。
  @req:r142
  规则: worktree 拓扑守卫——目标分支被他树持有时不自动合并
    finalize/archive 的自动合并 MUST 在 checkout 目标分支前经 `git worktree list` 感知拓扑：当合并目标分支已被其他 worktree 持有时 MUST 跳过自动合并（MUST NOT 试图在同一 worktree checkout 该分支；MUST NOT 在持有方 worktree 内代为执行合并），MUST 输出含持有方 worktree 路径的可执行手动命令指引（squash/ff 各自形式）与「归档文档仅落在当前分支、基准分支未收口」的显式 WARNING；随后 docs rename 与收尾照常完成，进程退出码不受影响。目标分支由当前 worktree 自己持有（即正在绑定分支上操作）时 MUST NOT 触发该守卫。
  @req:r114
  规则: spec scaffold 脚手架与书写指引
    llman sdd spec skeleton <capability>（命令名对齐 spec-format r133）MUST 生成合规的单载体骨架：仅创建扁平 llmanspec/specs/<capability>.feature（含 # language/# capability/# purpose/# scope 头注释与原生示例：一个 @req:<id> 规则块（TODO 自由文本描述）+ 块内嵌套示例场景，不生成顶层功能级示例）。skeleton MUST 通过 next-req-id 分配首个未占用 req_id 写入示例标签。--help 与错误提示 MUST 嵌入格式示例（头注释规则、@req 句柄规则），让 agent 一次写对。skeleton MUST 拒绝覆盖已存在的 spec 文件（除非 --force）。skeleton 创建的文件 MUST 能直接通过 validate --strict。
  @req:r115
  规则: 废除 change delta 与 change/specs 路径
    llman sdd change delta（及其 skeleton/add-req/add-scenario 子命令）MUST 被移除：任何模式下调用 MUST 以非零退出失败并提示 'change delta is removed; edit live specs on a feature branch via change start / attach'。change/specs/ 目录 MUST 不再被 determine_stage / archive / validate / parser 读取或扫描。archive MUST 不再合并 TOON delta（r113 的 ff-merge 取代之）。llman-sdd-sync skill 模板 MUST 被移除（统一 Git-native 后无 sync 概念）。project migrate MUST 为纯指引命令（不执行迁移）：--kind 仅接受 toon2features | specs-flatten，两者均只打印协作说明；`--kind spec-md2toon` / `--kind partitioned` / 隐藏别名 partition-migrate MUST 以非零退出报 unknown migration kind（见 spec-format r136/r141，零兼容），遗留 change/specs/ 或 *.feature.delta.toon 须人工清理或另开 change。
  @req:r124
  规则: proposal frontmatter schema 守卫
    llman sdd validate（单 change / --all / --specs 路径）MUST 对 active change 的 proposal.md frontmatter 进行未知字段检测：合法字段集为 depends_on、blocks、branch、base_sha、base_branch、needs_specs_change（base_sha/base_branch 语义见 r111，needs_specs_change 见 r1）。`baseSha`/`checkpointed`/`checkpoint_sha`/`checkpointSha`/`skip_specs_landing`/`rules_edit_acked`/`rules_touched`/`agent_acked` MUST 全部移除（无兼容读取，出现即 ERROR；旧项目由 migrations 升级工具一次性清理）。当 frontmatter 含合法集外的键（如 status、title、priority、author）时 MUST 报 ERROR（非 WARNING），错误消息 MUST 列出该未知字段名并提示合法字段集。changes/archive/ 下的 proposal MUST 免检（历史归档保持只读，零迁移成本）。determine_stage 行为 MUST 不变：stage 继续从磁盘 artifacts 与 attach binding 推断（r93 三态），MUST NOT 引入任何 frontmatter 字段（含 status）影响 stage；skip_specs_landing 仅影响 r1 的 readyToImplement，不影响 stage。
  @req:r127
  规则: 嵌套 change 递归发现与叶子 id 唯一
    llman sdd 对 active changes 的发现 MUST 在 llmanspec/changes/ 下递归查找自身含 proposal.md 的目录（跳过 archive/、以 . 开头的目录，且 MUST NOT 跟随符号链接）；Change id MUST 仍为叶子目录名且通过 validate_sdd_id（禁止路径分隔符）。同一活跃树内叶子 id MUST 唯一：发现阶段（list_changes/resolve 等）遇重复 MUST 以非零退出失败，stderr MUST 列出冲突的相对路径（相对 llmanspec/changes/），MUST NOT 静默丢弃条目。路径解析 MUST 经集中 resolve_change_dir（或等价），list/show/validate/graph 与 change 生命周期命令 MUST NOT 仅用 changes.join(id) 假定扁平布局。无 proposal.md 的中间分组目录 MUST NOT 被视为 change。
  @req:r128
  规则: list/show path 与 max-scan-depth
    llman sdd list --json 与 llman sdd show <change-id> --output json（及等价 JSON）对 active change MUST 附加 path 字段，值为相对 llmanspec/changes/ 的目录路径（嵌套如 some_a/c0；扁平时 path 等于叶子 id）。人读 show MUST 展示 path 行。扫描深度默认 MUST 为 8（相对 changes/：深度 1 为直子目录）；MUST 仅由顶层 CLI llman sdd --max-scan-depth <N> 覆盖（无 config.yaml 深度字段）；N < 1 MUST 非零退出。有效深度 MUST 对本进程内所有走发现路径的子命令一致生效。
  @req:r129
  规则: graph 无分组节点与 archive 扁平
    llman sdd graph 的 active 节点 MUST 仅来自含 proposal.md 的递归发现结果（叶子 id）；MUST NOT 将无 proposal.md 的直子/中间目录画成 partial 节点。depends_on/blocks 边仍按叶子 id。change archive（及 finalize 归档）MUST 将文档扁平移至 changes/archive/<date>-<leaf-id>/，MUST NOT 保留分组路径或 former_path。change new MUST NOT 因本需求新增 --group（分组目录由用户手动创建）。
  @req:r130
  规则: specs landing 单轨口径
    Specs landing 的 live specs 路径口径 MUST 收窄为 llmanspec/specs/**/*.feature（spec.toon 不再是合约载体，遗留文件被静默忽略）。0.5 起无锁定哈希门禁：change finalize/diff 与 validate --strict MUST NOT 执行 @human 场景哈希对比（机制已退役，见 spec-format r135），仅保留分支绑定纪律与结构门。

    场景: gate-accumulation-immune
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 acc-next 绑定于先前规则编辑已合入默认分支零推送的历史
      当 在非交互终端运行 llman sdd validate acc-next --strict --no-check
      那么 退出码为零
  @req:r29
  规则: change id 命名规约机读化（pattern 门禁 + 模板生成 + 取号预览）
    llmanspec/config.yaml MUST 支持可选 `change_id:` 段（`pattern`: 用户配置正则；`template`: minijinja 模板），未配置该段（或其字段）时 validate 与 change new 的行为 MUST 与无该段现状完全一致（零破坏）。配置 `pattern` 时 llman sdd validate MUST 对 active change id 做 full-match，违规 MUST 作为独立 ERROR 条目报告且消息 MUST 含违规 id 全文与 pattern 原文；校验范围 MUST 仅限 active `changes/` 发现产物——`changes/archive/` 与 llman 视野外的目录（如下游自定义 `delayed-changes/`）MUST NOT 被回溯强制。配置 `template` 时 llman sdd change new --from MUST 以模板渲染生成 id，预设变量 MUST 至少含 `llman_sdd_unique_id`、`verb`、`subject`、`date`：`llman_sdd_unique_id` = llmanspec/ 全树递归查重后的下一个未占用号，扫描范围 MUST 含 `changes/`、`changes/archive/` 与任意层级子目录中 change-id 形态的目录名（取号查重必须全树，漏扫即撞号）；压缩包冻结形态 best-effort 透视，透视工具缺失或失败时 MUST 输出 WARNING 列出该路径并提示人工核对；`verb` = 描述动词归一（add/update/remove/refactor/fix）或 `--verb` 显式指定；`subject` = 启发式消毒产物去除 verb 前缀；`date` = UTC 日期 `%Y-%m-%d`（对齐 archive 日期前缀先例）。模板引用未注入变量 MUST 以清晰错误失败。change new MUST 支持 `--dry-run`：只打印将生成的完整 id，MUST NOT 创建任何目录或文件。MUST 新增只读子命令 llman sdd change next-id：打印当前全树最大号与下一个可用号（即 `llman_sdd_unique_id` 的取值依据），MUST NOT 创建目录或修改任何文件。`--from` 的 help 文本 MUST 与实际行为一致。

    场景: change-id-pattern-violation-errors-active-only
      假如 已初始化含 change_id pattern 与 archive 形态存量的 sdd 项目且存在违规 active change "Weird Id!"
      当 在非交互终端运行 llman sdd validate --all --strict --no-check
      那么 退出码非零
      那么 stdout 包含 Weird Id!
      那么 stdout 不含 2026-09-13-c20-legacy

    场景: change-id-unconfigured-inert
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd validate --all --strict --no-check
      那么 退出码为零

    场景: change-new-from-template-dry-run-readonly
      假如 已初始化含 change_id template 的 sdd 项目
      当 在非交互终端运行 llman sdd change new --from "add user login" --dry-run
      那么 退出码为零
      那么 stdout 包含 -add-user-login
      而且 相对路径 llmanspec/changes/c1-add-user-login 不存在

    场景: next-id-scans-whole-tree-readonly
      假如 已初始化含 change_id 段且 delayed-changes 深层目录含更大号的 sdd 项目
      当 在非交互终端运行 llman sdd change next-id
      那么 退出码为零
      那么 stdout 包含 2620
  @req:r2
  规则: bdd.bindings 绑定源退役
    0.5 起 .feature 内标签不承载任何语义，bdd 段的 bindings 列表（kind=tags / kind=scenario-attrs）MUST 退役：MUST NOT 再作为 harness bound 口径的绑定源，声明与否 MUST NOT 改变 validate/list/show/review 的任何输出；历史 config.yaml 中残留的 bindings 键 MUST 被宽松剥离（解析结果不含该键，不报错）。bdd 段仅保留 runner 开关相关键（run_command 等，见 sdd-bdd-mode-compat r26）。
  @req:r3
  规则: bound/unbound 计数口径退役
    llman sdd list --specs / show / review MUST NOT 再输出 harness-bound / harness-unbound 拆分、harnessBoundCount / harnessUnboundCount 计数或 unbound 信号；场景统计仅保留原生 morphology 五键（spec-format r134），review 信号集收窄为 pending/stale/locked/validate（见 sdd-review r5）。
  @req:r42
  规则: valid_scope 路径存在性校验
    llman sdd validate（--specs、--all 及单 spec 路径）MUST 校验每个 spec 头注释 valid_scope 声明的文件/目录路径在磁盘上存在：缺失路径 MUST 作为独立失败项报告且消息 MUST 含缺失路径文本；--strict 时 MUST 报 ERROR 并以非零退出结束，非 strict 模式 MUST 报 WARNING 且不阻断其它校验项。本检查 MUST NOT 因 changes 或 skill 工件引入新的失败类别。

    场景: valid-scope-missing-path-fails-strict
      假如 已初始化含失效 scope 路径 spec 的 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd validate --specs --strict --no-check
      那么 退出码非零
      那么 stdout 包含 valid_scope
      那么 stdout 包含 docs/gone
  @req:r137
  规则: change diff 报告 commitCount 与多 commit 提示
    llman sdd change diff <id> MUST 报告自现算 merge-base（本地默认分支, HEAD；存储 base_sha 仅作审计、MUST NOT 参与范围计算）至 HEAD 的 commit 数量：人读输出 MUST 含计数行，--json MUST 输出合法 JSON 且含数值键 commitCount。llman sdd change finalize MUST 展示该计数，且当计数大于 1 时 MUST 打印不阻断执行的语义收敛建议提示。该行为 MUST NOT 引入任何新 config 字段。

    场景: diff-json-reports-commit-count
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 变更 diff-cnt 含 proposal design tasks 且 attach 状态为 "yes"
      当 在非交互终端运行 llman sdd change diff diff-cnt --json
      那么 退出码为零
      那么 stdout 为合法 JSON 且含 JSON 键 commitCount
      那么 stdout 的 JSON 键 commitCount 为数字
  @req:r138
  规则: list 停留时长可见性
    llman sdd list --json 的每个 change 对象 MUST 含 idleDays 数值键（自 change 目录最近活动时间起算的整数天，与 lastModified 同源）；文本人读输出 MUST 对 stage 为 draft 或 designed 的 change 追加停留天数标注。stage 为 full 及之后的 change MUST NOT 被追加该标注。

    场景: list-json-exposes-idle-days
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      当 在非交互终端运行 llman sdd list --json
      那么 退出码为零
      那么 stdout 为合法 JSON 且含 JSON 键 changes
      那么 stdout 的 JSON 键 changes.0.idleDays 为数字
  @req:r25
  规则: checkpoint 退役与 finalize 自动收口
    `llman sdd change checkpoint` MUST 被移除：任何调用 MUST 以非零退出失败并输出单行提示 `change checkpoint is removed; use change finalize`（对齐 r115 change delta 先例，无兼容别名）。`checkpointed`/`checkpoint_sha`/`checkpointSha` 字段 MUST 一并移除（见 r124）。`llman sdd change finalize` MUST 在收尾自动执行一次 git commit（未提交的实现 diff + frontmatter + archive rename 一次提交），提交说明 MUST 为固定式 `archive(sdd): <change-id>`；finalize MUST 提供不自动提交的 CLI 选项（--no-commit），此时跳过自动提交并输出手动 commit 指引，供 CI/脚本/pre-commit hook 冲突场景使用。自动提交失败 MUST 非零退出、保留 merge/rename 现场并提示 --no-commit 或手动 commit。finalize 幂等判定 MUST 基于「change 目录已在 changes/archive/」而非任何 frontmatter 存档字段。0.5 起全局 `--no-interactive` flag 移除（传入即 unknown option 用法错误）。change 分支上提交自由（分段 commit 或 finalize 单次收尾均可），skills/文档 MUST 如此说明。

    场景: finalize-auto-commits-archive
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 存在可收尾的完整 change fin-auto
      当 在非交互终端运行 llman sdd change finalize fin-auto --no-check
      那么 退出码为零
      那么 最近提交说明包含 archive(sdd): fin-auto

    场景: finalize-no-commit-leaves-dirty
      假如 已初始化 sdd 项目且 bdd 配置为 "on"
      而且 存在可收尾的完整 change fin-nocommit
      当 在非交互终端运行 llman sdd change finalize fin-nocommit --no-check --no-commit
      那么 退出码为零
      那么 最近提交说明不含 archive(sdd): fin-nocommit
      那么 工作区存在未提交改动
  @req:r28
  规则: 破坏性变更升级流程（migrations SOP）
    破坏性合约变更（移除/重命名 frontmatter 字段、命令、tag 或 stage 值域等）MUST 在同仓库提供 `migrations/v<from>-v<to>/` 升级目录，内含：README（升级 prompt：dry-run 报告 → --apply → 人工处理项 → `llman sdd validate --all --strict` 验证）与一次性升级脚本。脚本 MUST 默认 dry-run（`--apply` 才写）、MUST 跳过 `changes/archive/`（历史只读）、MUST 幂等；无法自动处理的项（如 `rules_edit_acked` 的真值语义）MUST 打印人工处理清单，MUST NOT 猜测。发布说明 MUST 指向对应版本区间的 migrations 目录。
