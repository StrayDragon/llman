```toon
kind: llman.sdd.delta
ops[5]{op,req_id,title,statement,from,to,name}:
  modify_requirement,r46,List 与 Show 阶段输出,"`llman sdd list` MUST 在变更列表输出中包含 stage 列，显示每个变更的完整度阶段（draft/specified/designed/full）。JSON 输出 MUST 包含 `stage` 字段。`llman sdd show <id>` 在展示 change 时 MUST 输出当前阶段标识：非 JSON（文本）模式在标题后打印 stage 行；JSON 模式 MUST 包含 `stage`（draft/specified/designed/full）、`artifacts`（已存在的 artifact 文件名列表）与 `readyToImplement`（当且仅当 stage 为 full 时为 true）字段。`stage` MUST 由 artifacts 存在性隐式推断（与 validate 的 `determine_stage` 同源），不引入额外状态文件。",null,null,null
  modify_requirement,r45,分级校验消息,"`llman sdd validate` MUST 根据 strict/non-strict 模式输出不同级别的完整度消息：non-strict 模式下缺少可选 artifact 时 MUST 输出可见的 INFO 级阶段提示（即使整体校验通过，draft 等非 full 阶段的提示仍 MUST 被打印给用户）；strict 模式下未达到 designed 阶段 MUST 输出 WARN（由 INFO 升级）。校验输出 MUST 在所有阶段都包含当前阶段标识，且不得在 valid 时吞掉阶段提示。",null,null,null
  modify_requirement,r33,SDD Apply Skill,"`llman-sdd-apply` skill MUST 指导 AI 助手实施 tasks.md 中的任务。skill MUST 读取变更的上下文文件（proposal、specs、design、tasks），按顺序实施未完成的任务，每完成一个任务后更新 tasks.md 中的 checkbox 状态。实施过程中遇到问题时 skill MUST 暂停并请求用户指导。skill MUST 在实施前执行阶段守卫：通过 `llman sdd show <id> --json` 读取权威 `stage`，当 stage 不为 `full` 时 MUST 拒绝实施并 STOP，引导用户使用 `llman-sdd-continue` 将变更长大到 full（proposal → specs → design → tasks）。draft 阶段的守卫文案 MUST 明确指出这是 draft 提案、需先补 specs → design → tasks。",null,null,null
  modify_requirement,r35,SDD Verify Skill,"`llman-sdd-verify` skill MUST 指导 AI 助手验证实现与变更 artifacts 的一致性。skill MUST 读取 specs 和 design，检查代码实现是否符合规范，识别不一致之处并提供修复建议。验证通过后 skill MUST 引导用户进行 archive。skill MUST 在验证前执行阶段守卫：通过 `llman sdd show <id> --json` 读取权威 `stage`，当 stage 不为 `full` 时 MUST 拒绝验证并 STOP，引导用户使用 `llman-sdd-continue` 将变更长大到 full 后再验证。",null,null,null
  modify_requirement,r32,SDD Continue Skill,"`llman-sdd-continue` skill MUST 指导 AI 助手继续未完成的变更，创建下一个待完成的 artifact。skill MUST 检查当前变更状态，识别已完成和待创建的 artifacts，按依赖顺序创建下一个 artifact。若所有 artifacts 已完成，skill MUST 引导用户进入 apply 阶段或 archive。当变更处于 draft 阶段（仅 proposal.md）时，skill MUST 显式提示这是 draft 提案，并指引按 specs → design → tasks 顺序长大到 full 后方可实现。",null,null,null
op_scenarios[8]{req_id,id,given,when,then}:
  r46,show-json-stage,"变更目录存在且包含 proposal.md","用户执行 `llman sdd show <id> --json --type change`","JSON 输出包含 stage、artifacts 与 readyToImplement 字段，且 readyToImplement 在 stage 非 full 时为 false"
  r46,show-text-stage,"变更目录存在","用户执行 `llman sdd show <id>`（非 JSON）","输出包含当前阶段标识（draft/specified/designed/full）"
  r45,non-strict-draft-可见,"变更处于 draft 阶段（仅 proposal.md）","用户执行 `llman sdd validate <change-id>`（非 strict）","输出包含可见的 INFO 级阶段提示，且不因整体校验通过（valid）而被吞掉"
  r45,strict-draft-warn,"变更处于 draft 阶段","用户执行 `llman sdd validate <change-id> --strict`","输出由 INFO 升级的 WARN 级阶段提示（strict 下为 ERROR）"
  r33,apply-draft-守卫,"变更处于非 full 阶段（如 draft，仅有 proposal.md）","用户调用 llman-sdd-apply","skill 通过 show 读取 stage 后拒绝实施并 STOP，引导使用 llman-sdd-continue 长大到 full"
  r33,apply-full-放行,"变更处于 full 阶段（proposal.md + specs/ + design.md + tasks.md）","用户调用 llman-sdd-apply","skill 正常实施 tasks.md 中的任务"
  r35,verify-non-full-守卫,"变更处于非 full 阶段","用户调用 llman-sdd-verify","skill 通过 show 读取 stage 后拒绝验证并 STOP，引导使用 llman-sdd-continue 长大到 full"
  r32,continue-draft-提示,"变更处于 draft 阶段（仅 proposal.md）","用户调用 llman-sdd-continue","skill 显式提示这是 draft 提案，并指引按 specs → design → tasks 顺序长大到 full 后方可实现"
```
