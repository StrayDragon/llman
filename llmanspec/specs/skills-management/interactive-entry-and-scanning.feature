# language: zh-CN
# 对应 spec: skills-management — llman skills 交互式扫描后直接进 agent→scope→skills 流程；
# 尊重忽略规则并跳过软链接解析；单一来源扫描不可配置；取消是安全 no-op；
# 项目范围菜单隐藏仅 user scope 已管理技能；scope 排序动态化。
功能: 交互入口、扫描与 scope 语义
  场景: 直接进入 agent 菜单
    假如 用户运行 llman skills
    当 管理器启动
    那么直接展示 Select which agent tools to manage
    而且不出现 Select mode

  场景: 既有 agent→scope→skills 流程保留
    假如 用户进入交互流程
    当 管理器执行
    那么按 agent → scope → skills 流程执行

  场景: 取消不产生变更
    假如 用户在确认前退出或返回
    当 管理器处理
    那么不修改任何目标链接
    而且不写入持久化状态文件

  场景: 忽略路径不被导入
    假如 skills_root 下存在被 .gitignore 排除的技能目录
    当 管理器扫描
    那么不导入该技能
    而且不创建托管记录

  场景: 软链接技能目录可被发现
    假如 skills_root 下技能目录是软链接且解析后含 SKILL.md
    当 管理器扫描
    那么将其作为可管理技能展示

  场景: 扫描单一根目录
    假如 skills_root 下存在含 SKILL.md 的技能目录
    当 管理器扫描
    那么将其作为可管理技能展示

  场景: 隐藏仅 user scope 已链接技能
    假如 用户进入 project/repo 管理且某技能在同 agent 的 user scope 已链接但当前 scope 未链接
    当 展示多选列表
    那么该技能不出现

  场景: 保留当前 scope 已链接技能
    假如 用户进入 project/repo 管理且某技能在当前 scope 已链接
    当 展示多选列表
    那么该技能继续显示

  场景: git 仓库内 project scope 优先
    假如 当前目录在 git 仓库内且非家目录
    当 用户选择 scope
    那么 project scope 选项排在 user scope 之前

  场景: 家目录或非 git 时 user scope 优先
    假如 当前目录是家目录或不在 git 仓库内
    当 用户选择 scope
    那么 user scope 选项排在 project scope 之前
