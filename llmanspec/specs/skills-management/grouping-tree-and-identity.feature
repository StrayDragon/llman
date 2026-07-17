# language: zh-CN
# 对应 spec: skills-management — skill_id 由 SKILL.md frontmatter name 经 slugify 得出（缺失回退目录名）；
# 默认 target 含 claude user/project 与 agents project；技能按目录名 . 推断分组（无 . 归 ungrouped）；
# 多选列表以树形结构展示分组节点（三态 + 搜索过滤）；技能项展示 skill_id (directory_name)；
# 预设仅运行时推断、仅交互模式。
功能: 标识规则、分组推断与树形多选
  @req:r34
  场景: name 缺失或非法时用目录名
    假如 SKILL.md 缺失 name 或含非法值
    当 计算 skill_id
    而且 那么使用目录名作为 skill_id

  @req:r34
  场景: name 需 slugify
    假如 SKILL.md 的 name 为 Slint GUI Expert
    当 计算 skill_id
    那么 skill_id 为 slint-gui-expert

  @req:r34
  场景: agents 为默认 target 之一
    假如 skills root 无 config.toml
    当 加载默认 targets
    而且 那么含 agents project（.agents/skills）以及 claude user/project

  @req:r34
  场景: 分组推断
    假如 技能目录名为 {grouped_name}
    当 推断分组
    而且 那么归入对应分组

  @req:r34
  场景: 无分组技能归 ungrouped
    假如 技能目录名不含 .
    当 推断分组
    而且 那么归入 ungrouped 分组

  @req:r34
  场景: 选择分组自动展开
    假如 用户在 skills 列表选择某分组节点
    当 管理器处理
    而且 那么将该分组对应技能集合加入最终选择集合

  @req:r34
  场景: 树形父子联动
    假如 用户在树形列表切换分组父节点
    当 管理器处理
    而且 那么该父节点下所有子技能同步选中或取消

  @req:r34
  场景: 重叠技能去重
    假如 用户同时选多个分组且含同一技能
    当 计算最终应用集合
    而且 那么该技能只保留一份

  @req:r34
  场景: 搜索过滤保留父节点
    假如 用户输入关键字仅匹配某技能
    当 列表过滤
    而且 那么仅显示匹配技能
    而且 而且其父分组保留显示

  @req:r34
  场景: 展示 skill_id 与目录名
    假如 某技能 skill_id 为 brainstorming 且目录名为 superpowers.brainstorming
    当 交互选项展示
    而且 那么显示为 brainstorming (superpowers.brainstorming)

  @req:r34
  场景: 命令行帮助不含预设参数
    假如 用户查看 llman skills --help
    当 检查帮助
    而且 那么不含 --preset、--save-preset、--list-presets 等参数
