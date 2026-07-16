# language: zh-CN
# 对应 spec: sdd-workflow r30-r36,r42 — 各 llman-sdd-* skill 的行为合约：
# archive 批量、explore 探索模式、continue 长大、apply 阶段守卫、ff 快速创建、verify 阶段守卫、
# sync 手动同步、propose 一次性创建工件。
功能: SDD skills 行为合约
  场景: 批量归档多个变更
    假如 用户调用 llman-sdd-archive 并提供多个 change IDs
    当 skill 执行
    而且 那么指导依次运行 archive run
    而且 而且结束后运行 validate --strict --no-interactive

  场景: 探索模式进入
    假如 用户调用 llman-sdd-explore
    当 skill 执行
    而且 那么进入探索模式
    而且 而且可阅读代码和创建 artifacts 但不实现功能

  场景: 探索模式退出引导
    假如 用户在探索模式中准备开始实现
    当 skill 引导
    而且 那么引导使用 llman-sdd-new-change、llman-sdd-ff 或 /llman-sdd:new 开始正式工作流

  场景: continue draft 提示
    假如 变更处于 draft 阶段
    当 用户调用 llman-sdd-continue
    而且 那么显式提示这是 draft 提案
    而且 而且指引按 specs → design → tasks 顺序长大到 full

  场景: apply draft 守卫
    假如 变更处于非 full 阶段
    当 用户调用 llman-sdd-apply
    而且 那么skill 通过 show 读取 stage 后拒绝实施并 STOP
    而且 而且引导使用 llman-sdd-continue 长大到 full

  场景: apply full 放行
    假如 变更处于 full 阶段
    当 用户调用 llman-sdd-apply
    而且 那么skill 正常实施 tasks.md 中的任务

  场景: ff 快速创建变更
    假如 用户调用 llman-sdd-ff <change-name>
    当 skill 执行
    而且 那么询问变更描述后依次创建 proposal、specs、design、tasks

  场景: ff 变更已存在提示继续
    假如 用户调用 llman-sdd-ff <change-name> 但该变更已存在
    当 skill 执行
    而且 那么提示使用 llman-sdd-continue 继续

  场景: verify non-full 守卫
    假如 变更处于非 full 阶段
    当 用户调用 llman-sdd-verify
    而且 那么skill 通过 show 读取 stage 后拒绝验证并 STOP
    而且 而且引导使用 llman-sdd-continue 长大到 full

  场景: sync 同步 delta specs 后验证
    假如 用户调用 llman-sdd-sync <change-name>
    当 skill 执行
    而且 那么提供可复现步骤指导手动同步 delta 到主 specs
    而且 而且合并后运行 llman sdd validate --specs 验证

  场景: propose 创建变更与工件
    假如 用户带变更描述（和/或 change id）调用 llman-sdd-propose
    当 skill 执行
    而且 那么助手创建 llmanspec/changes/<change-id>/，含 proposal.md、specs/**、tasks.md（需要时含 design.md）
