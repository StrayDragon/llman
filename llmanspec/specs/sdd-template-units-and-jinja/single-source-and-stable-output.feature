# language: zh-CN
# 对应 spec: sdd-template-units-and-jinja — 模板体系 MUST 避免保留易被误认为"共享真源"
# 的影子文件；共享内容真源 MUST 位于 templates/**/units/** 经 unit() 注入。
# 渲染产物 MUST 自包含且保持稳定输出顺序以减少维护 diff 噪声。
功能: 共享内容单一真源且渲染产物自包含稳定
  @req:r33
  场景: 共享内容仅由 units 承载
    假如 维护者需要更新多个 SDD 模板共享的一段提示内容
    当 改动落盘
    那么 该改动在单个 unit 文件中完成（位于 templates/**/units/**）

  @req:r33
  场景: 生成的 SKILL.md 自包含且无未解析注入标记
    假如 用户运行 llman sdd update-skills --no-interactive --tool {tool}
    当 生成完成
    那么 生成的 SKILL.md 包含完全渲染的内容
    而且 而且不存在未解析的注入标记（如 {{ unit(）

  @req:r33
  场景: 同一命令多次运行产物内容顺序一致
    假如 用户在无源码改动下运行同一生成命令两次
    当 比较产物
    那么 生成文件的内容顺序在两次运行间一致
