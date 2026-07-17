# language: zh-CN
# managed by llman sdd partition-migrate
功能: skills-management

  @req:r1
  场景: name 缺失或非法时用目录名
    假如 SKILL.md 缺失 name 或含非法值
    当 计算 skill_id
    那么 使用目录名作为 skill_id

  @req:r1
  场景: name 需 slugify
    假如 SKILL.md 的 name 为 Slint GUI Expert
    当 计算 skill_id
    那么 skill_id 为 slint-gui-expert

  @req:r1
  场景: agents 为默认 target 之一
    假如 skills root 无 config.toml
    当 加载默认 targets
    那么 含 agents project（.agents/skills）以及 claude user/project

  @req:r1
  场景: 分组推断
    假如 技能目录名为 {grouped_name}
    当 推断分组
    那么 归入对应分组

  @req:r1
  场景: 无分组技能归 ungrouped
    假如 技能目录名不含 .
    当 推断分组
    那么 归入 ungrouped 分组

  @req:r1
  场景: 重叠技能去重
    假如 用户同时选多个分组且含同一技能
    当 计算最终应用集合
    那么 该技能只保留一份

  @req:r1
  场景: 搜索过滤保留父节点
    假如 用户输入关键字仅匹配某技能
    当 列表过滤
    那么 仅显示匹配技能
    而且 其父分组保留显示

  @req:r1
  场景: 展示 skill_id 与目录名
    假如 某技能 skill_id 为 brainstorming 且目录名为 superpowers.brainstorming
    当 交互选项展示
    那么 显示为 brainstorming (superpowers.brainstorming)

  @req:r1
  场景: 命令行帮助不含预设参数
    假如 用户查看 llman skills --help
    当 检查帮助
    那么 不含 --preset、--save-preset、--list-presets 等参数

  @req:r1
  场景: 直接进入 agent 菜单
    假如 用户运行 llman skills
    当 管理器启动
    那么 直接展示 Select which agent tools to manage
    而且 不出现 Select mode

  @req:r1
  场景: 既有 agent→scope→skills 流程保留
    假如 用户进入交互流程
    当 管理器执行
    那么 按 agent → scope → skills 流程执行

  @req:r1
  场景: git 仓库内 project scope 优先
    假如 当前目录在 git 仓库内且非家目录
    当 用户选择 scope
    那么 project scope 选项排在 user scope 之前

  @req:r1
  场景: 家目录或非 git 时 user scope 优先
    假如 当前目录是家目录或不在 git 仓库内
    当 用户选择 scope
    那么 user scope 选项排在 project scope 之前

  @req:r1
  场景: 非交互冲突无策略报错
    假如 非交互模式下冲突且未传 --target-conflict
    当 命令执行
    那么 返回错误并提示使用 --target-conflict=overwrite|skip

  @req:r1
  场景: 覆盖断链 symlink
    假如 某 target 存在名为 skill_id 的断链 symlink 且用户希望启用该 skill
    当 冲突处理流程运行
    那么 可按选定冲突策略覆盖该断链 symlink

  @req:r1
  场景: init 填充版本
    假如 用户运行 llman sdd init
    当 生成 skills
    那么 metadata.version 为当前 CLI 版本

  @req:r1
  场景: update-skills 同步版本
    假如 用户运行 llman sdd update-skills
    当 更新完成
    那么 更新后的 skills 的 metadata.version 为当前 CLI 版本

  @req:r1
  场景: 主版本不匹配警告
    假如 skill 的 metadata.version 与当前 CLI 主版本（major.minor）不一致
    当 加载时
    那么 输出版本不匹配警告
    而且 不阻断 skill 加载或执行
