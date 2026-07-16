# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-template-units-and-jinja

  @req:r1
  场景: 共享内容仅由 units 承载
    假如 维护者需要更新多个 SDD 模板共享的一段提示内容
    当 改动落盘
    那么 该改动在单个 unit 文件中完成（位于 templates/**/units/**）

  @req:r1
  场景: 生成的 SKILL.md 自包含且无未解析注入标记
    假如 用户运行 llman sdd update-skills --no-interactive --tool {tool}
    当 生成完成
    那么 生成的 SKILL.md 包含完全渲染的内容
    而且 不存在未解析的注入标记（如 {{ unit(）

  @req:r1
  场景: 同一命令多次运行产物内容顺序一致
    假如 用户在无源码改动下运行同一生成命令两次
    当 比较产物
    那么 生成文件的内容顺序在两次运行间一致

  @req:r1
  场景: 维护者只改单个 unit 文件即可影响多个模板
    假如 维护者更新一个被多个 SDD 模板复用的提示单元
    当 改动落盘
    那么 改动仅在单个 unit 文件中完成
    而且 无需编辑无关模板

  @req:r1
  场景: locale 单元解析遵循确定性回退链
    假如 渲染器为 zh-Hans 解析某单元并回退到 en
    当 解析完成
    那么 按文档化的确定性回退链返回恰好一个解析出的单元源

  @req:r1
  场景: 缺失单元引用时渲染失败
    假如 模板引用了不存在的单元标识符
    当 执行渲染
    那么 渲染以非零退出并给出明确的缺失单元错误

  @req:r1
  场景: 缺失必需渲染变量时渲染失败
    假如 模板需要一个未提供的渲染变量
    当 执行渲染
    那么 渲染以非零退出并指明缺失的变量
