# language: zh-CN
# capability: spec-format
# purpose: 规范单轨 feature-as-spec 格式：每个 capability 以单个 .feature 为唯一规格事实源，@human 约束层与 @executable 验收层共存于同一文件，配套锁定哈希门禁、三态强制分级计数与 toon2features 一次性迁移。
# scope: src/sdd

功能: spec-format

  @req:r131 @human
  场景: 单轨规格事实源
    - 每个 capability 目录 MUST 仅以单个 .feature 文件作为规格唯一事实源（头注释元数据 + @human 约束场景 + @executable 验收场景）。运行时 MUST NOT 再读取 spec.toon：validate/list/show/context 遇遗留 spec.toon MUST 报 ERROR 并提示执行 llman sdd project migrate --kind toon2features。

  @req:r132 @human
  场景: tag 语法学
    - 场景 tag MUST 遵循保留字汇：@req:<id>（全库唯一约束标识）、@human（人类拥有的约束场景）、@manual（人审豁免）、@executable（harness 绑定验收）。validate MUST 校验 @req 悬空链接并判失败；无任何 @req 的 @executable 场景 MUST 给 WARNING（孤儿验收）；同一 @human 场景归一化后重复 MUST 判失败。可执行场景 MUST 位于 Feature 顶层（rstest-bdd scenarios! 不展开 Rule 块内场景）。

  @req:r133 @human
  场景: 头注释元数据
    - .feature 头部 MUST 携带 # capability、# purpose、# scope 三行注释元数据；scope 供 staleness 消费且路径 MUST 存在；llman sdd spec skeleton 生成的骨架 MUST 自带合法头注释与示例规则。

  @req:r134 @human
  场景: 三态强制分级计数
    - list --specs 与 show MUST 输出 rule 三态口径：ruleCount、ruleEnforcedCount、ruleManualCount、rulePendingCount、acceptanceCount、orphanAcceptanceCount；harnessBound/harnessUnbound/dualWrite 计数退役；bdd.bindings 配置段退役（tag 即声明，保留可选 override 以兼容下游自定义 step 库 tag）。

  @req:r135 @human
  场景: 锁定哈希门禁
    - 所有 @human 场景按规范化规则（id+name+description+steps 逐行 trim 尾随空白后 SHA-256）计算哈希；validate --strict 与 change finalize/checkpoint/diff MUST 对比 base_sha...HEAD 内哈希集合，任何增删改 MUST 报 ERROR，除非该 change proposal frontmatter 含 rules_edit_acked: true。rules_edit_acked MUST 加入 proposal frontmatter 合法字段集并同步 JSON Schema。

  @req:r136 @human
  场景: toon2features 一次性迁移
    - llman sdd project migrate --kind toon2features MUST 只处理遗留 spec.toon：requirements[] 无损转换为 @req:<id> @human 场景（statement 全文入 description）；同目录既有 *.feature 文件是活 harness 资产，MUST NOT 被读取、改写或删除，报告 MUST 计数 left 并提示按 r131 人工合并；已存在同名 <capability>.feature 的 capability MUST 跳过（保留 spec.toon，输出 skipped 警告，人工合并后重跑）。scenarios[] 行：given/when/then 任一非空且 req_id 配对 MUST 转写为 @req:<req_id> @human 场景（id 入场景标题，步骤关键字按项目 Gherkin 语言渲染：优先 config bdd.default_language，次 locale 映射，再次任一既有 .feature 的 # language: 头，兜底英文；单元格遗留关键字前缀 MUST 剥离；空列跳过，不得产生空步骤）；req_id 未配对 MUST 计入 dropped_unpaired 且不得转写（避免悬空 @req）；三列皆空 MUST 计入 dropped_notes；feature 列仅作历史记录不再分支。迁移 MUST 幂等，成功写出后删除 spec.toon，报告 MUST 区分 converted_from_toon / dropped_notes / dropped_unpaired / left 计数并列出规则三态初值。--kind spec-md2toon MUST 以非零退出拒绝并提示仅支持 toon2features。
  @executable
  @req:r136
  场景: migrate-toon2features-converts-and-cleans
    假如 已初始化含仅遗留 spec.toon 的 legacy capability 且 bdd 配置为 "off"
    当 运行 llman sdd project migrate --kind toon2features --yes
    那么 退出码为零
    那么 相对路径 llmanspec/specs/legacy/spec.toon 不存在
    那么 相对路径 llmanspec/specs/legacy/legacy.feature 存在


  @executable
  @req:r136
  场景: migrate-toon2features-keeps-features-and-converts-gwt-notes
    假如 已初始化含遗留 spec.toon 与既有 .feature 的 sample3 capability 且 bdd 配置为 "off"
    当 运行 llman sdd project migrate --kind toon2features --yes
    那么 退出码为零
    那么 相对路径 llmanspec/specs/sample3/spec.toon 不存在
    那么 相对路径 llmanspec/specs/sample3/sample3.feature 存在
    那么 相对路径 llmanspec/specs/sample3/sample3.feature 内容包含 @req:r1 @human
    那么 相对路径 llmanspec/specs/sample3/sample3.feature 内容包含 Given precondition ready
    那么 相对路径 llmanspec/specs/sample3/legacy-acc.feature 存在
    那么 stdout 包含 converted_from_toon 2
    那么 stdout 包含 dropped_unpaired 1
    那么 stdout 包含 dropped_notes 1
    那么 stdout 包含 left 1 legacy .feature


  @executable
  @req:r136
  场景: migrate-toon2features-skips-when-main-feature-exists
    假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd project migrate --kind toon2features --yes
    那么 退出码为零
    那么 相对路径 llmanspec/specs/sample/spec.toon 存在
    那么 stdout 包含 skipped


  @executable
  @req:r136
  场景: migrate-spec-md2toon-retired
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd project migrate --kind spec-md2toon
    那么 退出码非零
    那么 stderr 包含 toon2features


  @executable
  @req:r131
  场景: legacy-spec-toon-fails-with-pointer
    假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码非零
    那么 stderr 包含 toon2features


  @executable
  @req:r131
  场景: legacy-spec-toon-error-message-is-actionable
    假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码非零
    那么 stderr 包含 spec.toon
    那么 stderr 包含 project migrate


  @executable
  @req:r132
  场景: dangling-req-link-fails-strict
    假如 已初始化含无效 @req 的 sdd 项目且 bdd 配置为 on
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码非零
    那么 stderr 包含 @req


  @executable
  @req:r133
  场景: scaffold-emits-single-track-skeleton
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd spec skeleton demo-cap --force
    那么 退出码为零
    那么 相对路径 llmanspec/specs/demo-cap/demo-cap.feature 存在


  @executable
  @req:r134
  场景: list-specs-reports-rule-tier-counts
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd list --specs
    那么 退出码为零
    那么 stdout 包含 enforced
