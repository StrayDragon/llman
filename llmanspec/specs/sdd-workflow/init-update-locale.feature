# language: zh-CN
# 对应 spec: sdd-workflow r1-r3 — llman sdd init 创建 llmanspec 脚手架与受管 AGENTS.md 块；
# update 刷新受管块保留用户内容与 specs/changes 不变；locale 驱动模板加载（仅影响模板与 skills）。
功能: SDD 初始化、指令刷新与本地化模板加载
  场景: 初始化新项目创建目录结构与模板
    假如 用户在不存在 llmanspec/ 的目录执行 llman sdd init
    当 命令执行
    那么必要的目录结构与模板文件被创建

  场景: 初始化指定路径
    假如 用户执行 llman sdd init <path>
    当 命令执行
    那么在 <path> 下创建 llmanspec/ 结构与模板文件

  场景: 初始化时生成提示块
    假如 llman sdd init 生成 llmanspec/AGENTS.md
    当 生成完成
    那么文件含 LLMANSPEC:START 与 LLMANSPEC:END 包裹的提示块

  场景: 初始化时写入配置与 schema 头注释
    假如 用户执行 llman sdd init --lang en
    当 命令执行
    那么 config.yaml 被写入且 locale 为 en
    而且文件顶部含 yaml-language-server schema 头注释

  场景: 初始化时生成根 agents
    假如 llman sdd init 运行
    当 命令执行
    那么 repo 根目录 AGENTS.md 被创建或刷新受管块并指向 llmanspec/AGENTS.md

  场景: 已存在 llmanspec 目录时报错
    假如 用户在已有 llmanspec/ 的目录执行 llman sdd init
    当 命令执行
    那么返回错误且不做任何更改

  场景: openspec 共存时仅创建 llmanspec
    假如 openspec/ 已存在但 llmanspec/ 不存在
    当 执行 llman sdd init
    那么仅创建 llmanspec/
    而且不修改 openspec/

  场景: 更新刷新指令文件保留 specs 与 changes
    假如 用户执行 llman sdd update
    当 命令执行
    那么指令/模板文件被刷新
    而且现有 specs 与 changes 内容保持不变

  场景: 更新保留用户自定义内容
    假如 AGENTS.md 含用户自定义内容且含受管块
    当 执行 update
    那么仅替换受管块并保留其他内容

  场景: 更新根 agents 受管块
    假如 repo 根 AGENTS.md 存在且含 LLMANSPEC 受管块
    当 执行 update
    那么仅替换受管块并保留其他内容

  场景: locale 回退链
    假如 配置 locale 为 zh-Hans 但缺少对应模板
    当 解析模板
    那么按 zh-Hans → zh → en 顺序回退

  场景: locale 仅影响模板与 skills
    假如 config.yaml 设置 locale 为 zh-Hans
    当 生成
    那么 AGENTS.md 与 sdd skills 使用中文模板
