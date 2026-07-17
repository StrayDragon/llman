# language: zh-CN
# 对应 spec: sdd-structured-skill-prompts — 结构化协议 MUST 包含可执行的 ethics 治理字段，
# 缺失时校验失败；渲染产物 MUST 不含占位块或无效引导；涉及编写 spec.toon 的指引 MUST
# 使用 canonical TOON schema。
功能: 治理字段强制与无占位块及 TOON 指引
  @req:r1
  场景: 生成产物包含治理字段
    假如 生成新风格 SDD skills
    当 检查产物
    而且 那么包含治理字段（风险等级、禁止动作、必需证据、拒答合约、升级策略）

  @req:r1
  场景: 缺失治理字段时校验失败
    假如 某新风格技能制品遗漏了必需的 ethics 治理字段
    当 运行校验
    而且 那么以非零退出并给出明确的缺失字段诊断

  @req:r1
  场景: 生成产物无占位块
    假如 维护者运行 llman sdd update-skills --no-interactive --tool {tool}
    当 检查生成的任意 SKILL.md
    而且 那么不包含子串 Options: 或 <option

  @req:r1
  场景: 涉及 spec.toon 的指引使用 canonical schema
    假如 维护者审阅 templates/sdd/{locale}/skills/*.md
    当 阅读涉及编写 spec.toon 的指引
    而且 那么使用 canonical TOON 示例

  @req:r1
  场景: 生成的 skills 含 TOON 编辑指引
    假如 用户执行 llman sdd update-skills --no-interactive --all
    当 检查 SKILL.md
    而且 那么在适用处包含 canonical TOON 编辑指引

  @req:r1
  场景: 校验错误含 canonical 重写指引
    假如 用户因内容不符合 TOON schema 而报错
    当 查看指引或错误输出
    而且 那么包含将内容重写为 canonical TOON 的具体提示
    而且 而且不含遗留命令提示
