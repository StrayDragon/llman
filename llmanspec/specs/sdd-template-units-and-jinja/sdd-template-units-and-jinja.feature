# language: zh-CN
# capability: sdd-template-units-and-jinja
# purpose: 规范 SDD 模板单元的独立性、可发现性与 MiniJinja 注入渲染合约。
# scope: llmanspec/specs/sdd-template-units-and-jinja

功能: sdd-template-units-and-jinja

  @req:r33 @human
  场景: 共享内容单一真源且渲染产物自包含稳定
    - 对应 spec: sdd-template-units-and-jinja — 模板体系 MUST 避免保留易被误认为\共享真源\ 的影子文件；共享内容真源 MUST 位于 templates/**/units/** 经 unit() 注入。 渲染产物 MUST 自包含且保持稳定输出顺序以减少维护 diff 噪声。

  @req:r66 @human
  场景: 模板单元独立可发现并经 MiniJinja 注入渲染
    - 对应 spec: sdd-template-units-and-jinja — SDD 提示词组合 MUST 把可复用片段拆成独立的 模板单元文件（显式单元标识符 + 按 locale 的确定性查找）；渲染 MUST 基于 MiniJinja 注入， 且在缺失单元引用或必需变量时快速失败。

  @req:r75 @human
  场景: 渲染产物宿主编号连续性
    - 模板单元经 unit() 注入宿主文档时 MUST NOT 破坏宿主有序步骤列表的编号连续性：渲染产物中任一有序列表 MUST 保持 1..N 连续（MUST NOT 出现 1→3 式断档或重复编号）。just check-sdd-templates MUST 对全部渲染 skill 产物执行该连续性断言。

  @req:r141 @human
  场景: 生成式渲染变量取代静态命令单元
    - init --update 的渲染上下文 MUST 支持生成式变量（如 sdd_command_reference）：其内容由渲染进程从 CLI 命令树与 i18n 现算，不落模板仓库；模板通过 {{ sdd_command_reference }} 引用。静态手写单元与生成式变量 MUST NOT 对同一内容双轨并存（如 skills/sdd-commands 静态单元 MUST 已删除）。生成式变量的装配 MUST 遵循 r66 的注入语义：变量缺失时渲染快速失败，one-liner 级别的缺 key 回退（clap about）不视为缺失。
