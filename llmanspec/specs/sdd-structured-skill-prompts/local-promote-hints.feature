# language: zh-CN
# 对应 spec: sdd-structured-skill-prompts r98 — BDD-on 收尾提示不默认导向 PR/push。
# 三条行为：apply-cycle 技能含本地 merge 步骤 + 禁止默认 push/PR；
# finalize 成功 stdout 追加本地 merge 提示；
# validate <change> 失败时 BDD-on 下不诱导编写 delta。
功能: BDD-on 收尾提示不默认导向 PR/push

  @req:r98
  场景: apply-cycle 技能含本地合回默认分支步骤
    假如 项目 config 含 bdd 段
    当 检查生成的 llman-sdd-apply-cycle/SKILL.md
    那么 工作流含「本地合回默认分支」步骤且示例 git merge --ff-only
    而且 硬约束声明未获用户明确要求时禁止 git push 与 gh pr create/merge

  @req:r98
  场景: BDD-on finalize 成功提示指向本地 merge
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change finalize <已 attach 的 change>
    那么 stdout 包含本地 merge 进默认分支的 next-step 提示
    而且 stdout 标注 push / hosting PR 为可选

  @req:r98
  场景: BDD-on validate change 失败不诱导编写 delta
    假如 已初始化 sdd 项目且 bdd 配置为 "on" 且存在格式不完整的 change
    当 在非交互终端运行 llman sdd validate <change> --no-check
    那么 退出码非零
    那么 stderr 不含 Ensure change has deltas
    而且 stderr 指向 live specs 与 attach/finalize

  @executable @req:r99
  场景: change new --from 从描述生成合法 id
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change new --from "add user login"
    那么 退出码为零
    那么 stdout 包含 proposal.md
    那么 stdout 包含 derived change id

  @executable @req:r99
  场景: change new --from 冲突既有 change 时失败
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change new add-user-login
    而且 在非交互终端运行 llman sdd change new --from "add user login"
    那么 退出码非零
    那么 stderr 包含 --force

  @req:r99
  场景: 轻量 draft 路径不询问 change id
    假如 用户说「draft 提案：加一个用户登录功能」且未提供 change id
    当 agent 走 llman-sdd-propose 的轻量 draft 路径
    那么 直接调用 llman sdd change new 生成 id 并建 proposal.md
    而且 不询问用户确认 change id
    而且 告知用户已生成的 id（可应要求修改）
