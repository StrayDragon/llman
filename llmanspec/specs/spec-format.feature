# language: zh-CN
# capability: spec-format
# purpose: 规范原生 Gherkin 分层单轨格式：每个 capability 以单个 .feature 为唯一规格事实源（扁平或目录双布局），功能→规则→场景分层——规则块挂 @req 句柄、描述自由文本、嵌套场景为验收示例、顶层场景为功能级示例，历史标签惰性，配套头注释元数据、morphology 计数与遗留双轨（spec.toon/标签轨）静默忽略及协作指引。
# scope: src/external_command.rs

功能: spec-format

  @req:r131
  规则: 单轨规格事实源与布局
    每个 capability MUST 以恰好一个 .feature 文件作为规格唯一事实源（头注释元数据 + 功能→规则→场景原生分层：规则块挂 @req 句柄，验收示例为块内嵌套场景，顶层场景为功能级示例）。capability 布局 MUST 二选一：扁平 llmanspec/specs/<cap>.feature（cap id = 文件 stem；新默认，spec skeleton 与 authoring 走此形态）或目录 llmanspec/specs/<cap>/（cap id = 目录名；主文件为同名 .feature，目录内可含其它 .feature 作为草稿/资产，不计入主文件）。同一 cap id 的扁平与目录两种布局并存 MUST 判为冲突 ERROR（报告两个路径）。目录内非同名 .feature MUST 仅给 WARNING（不阻断 validate/list/show）。命名约定（文件名 vs 目录名 vs # capability: 头）由项目自约定，CLI 不强制；header 与 cap id 不一致 MUST 仅给 WARNING。遗留 spec.toon MUST 被静默忽略（不读取、不报错、不计数）：validate/list/show/context 遇之照常以同目录 .feature 为准；project migrate 仅输出协作指引、不执行迁移（见 r136/r141）。

    场景: legacy-spec-toon-silently-ignored
      假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd list --specs
      那么 退出码为零
      那么 stdout 不含 spec.toon

    场景: specs-flat-file-is-a-capability
      假如 已初始化含扁平 capability 的 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd list --specs
      那么 退出码为零
      那么 stdout 包含 flatcap
      当 在非交互终端运行 llman sdd show flatcap
      那么 退出码为零
      当 在非交互终端运行 llman sdd validate flatcap --strict --no-check
      那么 退出码为零

    场景: flat-and-dir-collision-errors
      假如 已初始化含同 id 扁平与目录冲突的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd list --specs
      那么 退出码非零
      那么 stderr 包含 llmanspec/specs/foo.feature
      那么 stderr 包含 llmanspec/specs/foo/foo.feature

    场景: multi-feature-dir-warns-not-fails
      假如 已初始化含多 .feature 目录 capability 的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd validate multi --strict --no-check
      那么 退出码为零
      那么 stderr 包含 WARNING
      当 在非交互终端运行 llman sdd show multi
      那么 退出码为零

    场景: dir-without-main-resolves-single
      假如 已初始化含异名单文件目录 capability 的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd show solocap
      那么 退出码为零
      当 在非交互终端运行 llman sdd validate solocap --strict --no-check
      那么 退出码为零
  @req:r141
  规则: specs-flatten 协作指引（不执行迁移）
    llman sdd project migrate --kind specs-flatten MUST 仅打印目录布局→扁平布局的手工迁移协作说明并成功返回，MUST NOT 执行 git mv、改写 # scope: 或删除任何文件。目录布局（llmanspec/specs/<cap>/<cap>.feature）仍是 r131 认可的合法布局，是否平铺由项目自行决定；协作说明 MUST 覆盖候选判定（纯同名单文件目录）与需人工处理形态（conflict/legacy/multi/aux/misnamed）。重复执行 MUST 幂等（纯打印，无副作用）。

    场景: flatten-prints-guidance-only
      假如 已初始化含单文件目录 capability 的 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd project migrate --kind specs-flatten
      那么 退出码为零
      那么 相对路径 llmanspec/specs/scoped/scoped.feature 存在
      那么 stdout 包含 specs-flatten
  @req:r132
  规则: 原生分层结构与标签惰性
    .feature MUST 按功能→规则→场景原生分层：每个需求 = `规则:` 块（`@req:<id>` 挂块头标签为唯一需求句柄，标题 + 自由文本描述，不强制 MUST/SHALL 词），验收示例 = 块内嵌套 `场景:`（GWT 步骤）；顶层 `场景:` 为功能级示例（无句柄、不告警）。validate MUST 判 ERROR：规则块缺 `@req:<id>`、跨 capability 全局重复 req_id、零规则 capability；裸规则（无嵌套场景）仅计入聚合计数 INFO（--include-info 可见）。历史标签 @human/@executable/@manual/@rule MUST NOT 再承载任何语义（解析不报错、不校验、不教学）；不再有互斥/豁免/悬空链接等旧机制。

    场景: duplicate-req-id-fails-strict
      假如 已初始化含跨 capability 重复 req_id 的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd validate sample --strict --no-check
      那么 退出码非零
      那么 stderr 包含 req_id
  @req:r133
  规则: 头注释元数据
    .feature 头部 MUST 携带 # capability、# purpose、# scope 三行注释元数据；scope 供 staleness 消费且路径 MUST 存在；llman sdd spec skeleton 生成的骨架 MUST 自带合法头注释与原生示例（规则块 + 嵌套场景，扁平 llmanspec/specs/<capability>.feature 形态）。

    场景: scaffold-emits-single-track-skeleton
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd spec skeleton demo-cap --force
      那么 退出码为零
      那么 相对路径 llmanspec/specs/demo-cap.feature 存在
  @req:r134
  规则: 原生 morphology 计数
    list --specs 与 show MUST 输出原生 morphology 五键：ruleCount、ruleEnforcedCount（含嵌套场景的规则）、rulePendingCount（裸规则）、acceptanceCount（嵌套场景）、featureScenarioCount（顶层功能级示例）。ruleManualCount、orphanAcceptanceCount、harnessBoundCount、harnessUnboundCount、dualWriteCount 与 bdd.bindings 配置段 MUST 退役（不再输出、不再读取；bdd 段仅保留 runner 开关相关键）。

    场景: list-specs-reports-rule-tier-counts
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd list --specs
      那么 退出码为零
      那么 stdout 包含 enforced
  @req:r135
  规则: 锁定哈希门禁退役
    0.5 起不再有 @human 场景锁定哈希门禁：validate --strict 与 change finalize/diff MUST NOT 执行任何哈希对比，也不再输出锁定编辑 WARNING；review 的 locked 信号 MUST 恒为 0。规则锁定保护退化为 Git 分支纪律：live specs 仅在绑定分支编辑（见 sdd-workflow r111/r130）；归档历史保护由 archive freeze/thaw 的 7z 冷备承担（外部 llman-sdd 所有）。确认元数据保持移除状态：frontmatter 字段 rules_touched / agent_acked、@agent tag、--yes 锁定确认语义 MUST NOT 再被读取或生成。

    场景: review-locked-signal-is-zero
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd review --json
      那么 退出码为零
      那么 stdout 的 JSON 键 signals 含 kind 为 locked 的条目且 count 为 0

    场景: validate-omits-locked-hash-warnings
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd validate --specs --strict --no-check
      那么 stderr 不含 locked
  @req:r136
  规则: toon2features 协作指引（不执行迁移）
    llman sdd project migrate MUST 不执行任何文件迁移：--kind toon2features MUST 仅打印手工迁移协作说明（agent/人类分工：requirements 表每行 → @req 规则块、scenarios 表 GWT 行 → 嵌套场景、既有 <cap>.feature 人工合并不覆盖、迁移后 validate 收口）并成功返回；--kind specs-flatten 同理（见 r141）；其它 --kind 值（含 spec-md2toon、partitioned 及隐藏别名）MUST 以非零退出报 unknown migration kind 并提示合法值（toon2features | specs-flatten）。遗留 spec.toon 在运行时被静默忽略，不阻断任何命令。

    场景: migrate-toon2features-prints-guidance-only
      假如 已初始化含遗留 spec.toon 的 legacy capability 且 bdd 配置为 "off"
      当 运行 llman sdd project migrate --kind toon2features
      那么 退出码为零
      那么 相对路径 llmanspec/specs/legacy/spec.toon 存在
      那么 stdout 包含 规则

    场景: migrate-unknown-kind-rejected
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd project migrate --kind spec-md2toon
      那么 退出码非零
      那么 stderr 包含 specs-flatten
