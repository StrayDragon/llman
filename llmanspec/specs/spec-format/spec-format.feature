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
    - llman sdd project migrate --kind toon2features MUST 把 requirements[] 无损转换为 @req/@human 场景（statement 全文入 description）、丢弃 feature:false note 行、合并同目录既有 .feature 并保持幂等；迁移报告 MUST 列出三态初值。--kind spec-md2toon MUST 以非零退出拒绝并提示仅支持 toon2features。
