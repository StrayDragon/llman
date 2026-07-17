# language: zh-CN
# 对应 spec: sdd-workflow r4-r6,r19,r21-r22,r24,r28-r29,r90 — update-skills 生成结构化技能与 workflow
# commands；模板用 MiniJinja 单元注入复用；sdd 命令范围；模板版本元信息；skills 符合 SKILL.md
# 规范且不暴露 --force；multi-tool --path 安全；workflow command bindings 仅 Claude；
# 废弃 llman-sdd-* 在 init --update 时按候选集先删再写。
功能: Skills 生成、模板复用与命令范围
  场景: 生成包含 specs-compact skill
    假如 用户执行 llman sdd update-skills --no-interactive --tool codex
    当 生成完成
    而且 那么目标路径下存在 llman-sdd-specs-compact/SKILL.md

  场景: 结构化协议在已生成技能中可见
    假如 用户执行 llman sdd update-skills --no-interactive --all
    当 生成完成
    而且 那么生成的技能均含统一结构化章节

  场景: 模板单元缺失时报错
    假如 模板声明的单元在当前 locale 与回退链中都不存在
    当 渲染
    而且 那么报错并退出非零

  场景: 模板单元注册冲突时报错
    假如 同一渲染上下文存在重复单元标识符
    当 渲染
    而且 那么报错并拒绝继续渲染

  场景: 帮助文本含 import export convert
    假如 用户执行 llman sdd --help
    当 查看帮助
    而且 那么帮助含 import、export 与 convert

  场景: 旧命名 migrate 不可用
    假如 用户执行 llman sdd migrate --from openspec
    当 命令执行
    而且 那么返回未知子命令错误

  场景: 模板版本一致性检查
    假如 维护者运行 just check-sdd-templates
    当 命令执行
    而且 那么在缺失元信息、缺少 locale 模板或版本不一致时退出非零

  场景: skills name 与目录一致
    假如 update-skills 写入 llman-sdd-archive/SKILL.md
    当 生成完成
    而且 那么frontmatter name 为 llman-sdd-archive

  场景: skills description 非空
    假如 update-skills 生成任意 SKILL.md
    当 生成完成
    而且 那么frontmatter description 为非空字符串

  场景: skills 不含 force
    假如 维护者运行 update-skills
    当 生成完成
    而且 那么生成的 SKILL.md 不提及 --force

  场景: multi-tool 单 path 被拒
    假如 用户运行 update-skills --no-interactive --all --path ./skills-out
    当 命令执行
    而且 那么以非零退出并解释如何安全地按 tool 生成

  场景: 仅生成 workflow commands
    假如 用户执行 update-skills --no-interactive --tool claude --commands-only
    当 生成完成
    而且 那么仅存在命令文件（new/continue/ff/apply/verify/sync/archive/explore/onboard）

  场景: codex 不生成 command prompts
    假如 用户执行 update-skills --no-interactive --tool codex
    当 生成完成
    而且 那么不在 .codex/prompts/ 下生成 llman-sdd-*.md

  @executable @req:r90
  场景: init --update 清理废弃 llman-sdd 技能
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    假如 项目中存在技能目录 llman-sdd-solidify
    假如 项目中存在技能目录 my-custom-skill
    当 在非交互终端运行 llman sdd init --update
    那么 退出码为零
    那么 stderr 包含 Cleaned up stale skill
    那么 相对路径 .agents/skills/llman-sdd-solidify 不存在
    那么 相对路径 .agents/skills/my-custom-skill 存在
    那么 相对路径 .agents/skills/llman-sdd-explore 存在

  @executable @req:r90
  场景: extra_skills 扩展候选不被清理
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    假如 项目 extra_skills 包含 llman-sdd-sync
    当 在非交互终端运行 llman sdd init --update
    那么 退出码为零
    那么 相对路径 .agents/skills/llman-sdd-sync 存在
