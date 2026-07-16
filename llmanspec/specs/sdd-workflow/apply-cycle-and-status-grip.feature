# language: zh-CN
# 对应 spec: sdd-workflow r48-r50 — apply-cycle 补充技能（disable-model-invocation、不在
# available_skills、无 Claude workflow 命令）；apply-cycle 单闭环（status TOON 为唯一输入）；
# status 作为 agent 抓手。
功能: Apply-Cycle 补充技能与 status 抓手
  场景: apply-cycle 含 disable-invocation
    假如 用户运行 update-skills --all --no-interactive
    当 生成完成
    而且 那么生成的 SKILL.md 含 frontmatter disable-model-invocation: true

  场景: apply-cycle 不在 available_skills
    假如 扫描 .agents/skills/llman-sdd-apply-cycle/
    当 检查 available_skills
    而且 那么该 skill 不在 available_skills 中

  场景: apply-cycle 不在 Claude commands
    假如 用户运行 update-skills --tool claude
    当 生成完成
    那么 .claude/commands/ 下无 llman-sdd-apply-cycle.md

  场景: apply-cycle 解析 status TOON
    假如 用户触发 /skill:llman-sdd-apply-cycle <id>
    当 skill 执行
    而且 那么运行 llman sdd status <id>
    而且 而且解析 TOON tasks[] 找出未完成任务

  场景: status 作为抓手模式
    假如 未来某 skill 使用 llman sdd status 作为输入
    当 该 skill 执行
    而且 那么遵循同一模式：一个 status TOON 取代多次文件读取
