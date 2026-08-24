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
    - 对应 spec: sdd-template-units-and-jinja — SDD 提示词组合 MUST 把可复用片段拆成独立的 模板单元文件（显式单元标识符 + 按 locale 的确定性查找）；渲染 MUST 基于 MiniJinja 注入，
