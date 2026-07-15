# language: zh-CN
# 对应 spec: sdd-structured-skill-prompts — SDD 技能模板 MUST 采用统一结构化提示协议
# （Context/Goal/Constraints/Workflow/Decision Policy/Output Contract），经模板单元注入组装；
# 且 MUST 自包含，不得要求调用外部技能作为前置依赖。
功能: 结构化提示协议自包含且无外部硬依赖
  场景: 协议经共享单元注入而非手工拷贝
    假如 维护者检查 templates/sdd/{locale}/skills/*.md
    当 协议章节被组装
    那么通过共享模板单元注入
    而且不是手工重复拷贝

  场景: 注入后结构化章节完整可见
    假如 用户执行 llman sdd update-skills --no-interactive --all
    当 生成完成
    那么 生成产物中可见完整结构化章节
    而且顺序一致

  场景: 生成内容不引用外部技能作为必需步骤
    假如 用户执行 llman sdd update-skills --all
    当 检查生成的 SKILL.md
    那么不包含先调用外部技能再执行的硬依赖指令
