# language: zh-CN
# 对应 spec: sdd-structured-skill-prompts r96 — skill 模板按 bdd_enabled 条件渲染；
# BDD-on/off 产物路径与交叉引用不同；保留 mermaid；optional skill 引用受 extra_skills 门控。
功能: Skill 模板 BDD 模式条件渲染
  @req:r96
  场景: BDD-on 渲染不以 delta specs 为 propose 主产物
    假如 项目 config 含 bdd 段
    当 运行 update-skills 生成 llman-sdd-propose/SKILL.md
    那么 frontmatter description 不将 delta specs 表述为唯一主产物
    而且 正文保留 mermaid pipeline 图
    而且 metadata.llman_sdd.bdd_mode 为 on

  @req:r96
  场景: BDD-off 渲染不以 finalize 为必经
    假如 项目 config 不含 bdd 段
    当 运行 update-skills 生成 llman-sdd-archive/SKILL.md
    那么 正文不将 attach/checkpoint/finalize 表述为必经路径
    而且 metadata.llman_sdd.bdd_mode 为 off

  @req:r96
  场景: 未启用 continue 时 propose 不强制推荐该 skill
    假如 项目 extra_skills 未包含 llman-sdd-continue
    当 检查生成的 llman-sdd-propose/SKILL.md
    那么 若提及已有 change 的下一步则给出不依赖 continue 的替代指引
