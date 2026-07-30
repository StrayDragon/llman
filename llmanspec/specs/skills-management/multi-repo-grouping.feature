# language: zh-CN
# 对应 spec: skills-management r126 — 技能发现按 repo 记录来源元数据；TUI 默认分组改为 repo 源（顶层），
# 目录名 . 前缀降为次级；单 repo 退化为现状；跨 repo 同名按列表顺序首个生效 + 冲突警告；
# / 搜索扩展匹配 repo name 与路径简称。
# 这些场景描述内部行为（扫描、分组、去重），不适合 CLI 子进程驱动 → 保持 fast mode，
# 不标 @executable；由单元测试覆盖（grouped_skill_options / dedupe_skills / discover_skills）。
功能: 多 repo 来源元数据与 TUI 按源分组
  @req:r126
  场景: 技能发现携带 repo 来源元数据
    假如 skills.repo 含多个仓库源
    当 管理器扫描
    而且 那么每个技能记录其所属 repo 的 repo_id 与 repo_name

  @req:r126
  场景: TUI 默认按 repo 源顶层分组
    假如 skills.repo 含多个仓库源
    当 展示多选列表
    而且 那么顶层分组维度为 repo 源
    而且 而且目录名 . 前缀降为次级分组维度

  @req:r126
  场景: 单 repo 时不显示 repo 分组头
    假如 仅配置单一 repo
    当 展示多选列表
    而且 那么不显示 repo 分组头
    而且 而且行为与单源发现一致

  @req:r126
  场景: 跨 repo 同名按列表顺序首个生效
    假如 多个 repo 含相同 skill_id
    当 计算最终技能集合
    而且 那么按 repo 列表顺序首个生效
    而且 而且对其余重复项输出冲突警告

  @req:r126
  场景: 搜索扩展匹配 repo name 与路径简称
    假如 用户输入关键字匹配某 repo 的 name 或路径简称
    当 列表过滤
    而且 那么该 repo 下技能保留显示
