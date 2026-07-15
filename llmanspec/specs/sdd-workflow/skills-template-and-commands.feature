# language: zh-CN
# 对应 spec: sdd-workflow r4-r6,r19,r21-r22,r24,r28-r29 — update-skills 生成结构化技能与 workflow
# commands；模板用 MiniJinja 单元注入复用；sdd 命令范围；模板版本元信息；skills 符合 SKILL.md
# 规范且不暴露 --force；multi-tool --path 安全；workflow command bindings 仅 Claude。
功能: Skills 生成、模板复用与命令范围
  场景: 生成包含 specs-compact skill
    假如 用户执行 llman sdd update-skills --no-interactive --tool codex
    当 生成完成
    那么目标路径下存在 llman-sdd-specs-compact/SKILL.md

  场景: 结构化协议在已生成技能中可见
    假如 用户执行 llman sdd update-skills --no-interactive --all
    当 生成完成
    那么生成的技能均含统一结构化章节

  场景: 模板单元缺失时报错
    假如 模板声明的单元在当前 locale 与回退链中都不存在
    当 渲染
    那么报错并退出非零

  场景: 模板单元注册冲突时报错
    假如 同一渲染上下文存在重复单元标识符
    当 渲染
    那么报错并拒绝继续渲染

  场景: 帮助文本含 import export convert
    假如 用户执行 llman sdd --help
    当 查看帮助
    那么帮助含 import、export 与 convert

  场景: 旧命名 migrate 不可用
    假如 用户执行 llman sdd migrate --from openspec
    当 命令执行
    那么返回未知子命令错误

  场景: 模板版本一致性检查
    假如 维护者运行 just check-sdd-templates
    当 命令执行
    那么在缺失元信息、缺少 locale 模板或版本不一致时退出非零

  场景: skills name 与目录一致
    假如 update-skills 写入 llman-sdd-archive/SKILL.md
    当 生成完成
    那么frontmatter name 为 llman-sdd-archive

  场景: skills description 非空
    假如 update-skills 生成任意 SKILL.md
    当 生成完成
    那么frontmatter description 为非空字符串

  场景: skills 不含 force
    假如 维护者运行 update-skills
    当 生成完成
    那么生成的 SKILL.md 不提及 --force

  场景: multi-tool 单 path 被拒
    假如 用户运行 update-skills --no-interactive --all --path ./skills-out
    当 命令执行
    那么以非零退出并解释如何安全地按 tool 生成

  场景: 仅生成 workflow commands
    假如 用户执行 update-skills --no-interactive --tool claude --commands-only
    当 生成完成
    那么仅存在命令文件（new/continue/ff/apply/verify/sync/archive/explore/onboard）

  场景: codex 不生成 command prompts
    假如 用户执行 update-skills --no-interactive --tool codex
    当 生成完成
    那么不在 .codex/prompts/ 下生成 llman-sdd-*.md
