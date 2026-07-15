# language: zh-CN
# 对应 spec: sdd-workflow r12-r13,r15,r25,r38-r39,r43-r47 — validate 支持 task 二态与 strict_defer；
# 校验提示增强；staleness 校验含 base-ref 安全校验；阶段检测（draft/specified/designed/full）；
# Design 前置约束；分级校验消息；list/show 阶段输出；TOON 编辑命令；轻量 spec 元数据。
功能: 校验、staleness、阶段检测与元数据
  场景: task 仅解析为 Completed 或 Pending
    假如 task 行为 - [x] 或 - [ ]
    当 执行 validate
    那么仅解析为 Completed 或 Pending

  场景: cancelled 现为 pending
    假如 遗留 tasks.md 含 - [ ] task (cancelled - reason)
    当 执行 validate
    那么该 task 被解析为 Pending

  场景: strict 下 pending 为 error
    假如 strict_defer 为 true 且存在 pending
    当 执行 validate
    那么该 pending 报 Error

  场景: 不再含 orphans 子命令
    假如 用户执行 llman sdd --help
    当 查看帮助
    那么不再包含 orphans 子命令

  场景: 拒绝 option-like base ref
    假如 LLMANSPEC_BASE_REF=-c
    当 用户运行 llman sdd validate --strict
    那么命令失败并报告非法 base ref
    而且未将 -c 作为 git option 执行

  场景: 生成 delta 骨架并添加 op
    假如 维护者创建新 change 目录并需要添加 delta requirement
    当 维护者使用 CLI
    那么可生成 delta spec 骨架并添加 op，无需手工编辑表格

  场景: agent 廉价获取 spec feature 名
    假如 agent 组装 prompt 时需要 spec 的 feature 名/purpose
    当 agent 调用 llman sdd show <spec-id> --type spec --json --meta-only
    那么返回轻量元数据

  场景: 仅有 proposal 时阶段为 draft
    假如 变更目录仅含 proposal.md
    当 用户执行 llman sdd validate <change-id>
    那么输出含阶段标识 draft

  场景: 完整变更阶段为 full
    假如 变更目录含 proposal.md + specs/ + design.md + tasks.md
    当 用户执行 llman sdd validate <change-id>
    那么输出含阶段标识 full

  场景: tasks 无 design 报 ERROR
    假如 变更目录含 proposal.md、specs/、tasks.md 但缺 design.md
    当 用户执行 llman sdd validate <change-id>
    那么校验失败并输出 ERROR 级别消息

  场景: non-strict draft 阶段提示可见
    假如 变更处于 draft 阶段
    当 用户执行 validate（非 strict）
    那么输出含可见的 INFO 级阶段提示
    而且不因整体 valid 被吞掉

  场景: show json 含 stage 字段
    假如 变更目录存在且含 proposal.md
    当 用户执行 llman sdd show <id> --json --type change
    那么JSON 含 stage、artifacts 与 readyToImplement
    而且readyToImplement 在 stage 非 full 时为 false
