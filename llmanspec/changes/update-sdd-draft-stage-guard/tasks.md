# Tasks: update-sdd-draft-stage-guard

- [ ] **R1: `show` 暴露 stage 字段**
  - 定位 `llman sdd show` change 分支的 JSON 结构体（`src/sdd/command.rs` 中 change 输出结构，当前 `{id, title, deltaCount, deltas}`）
  - 复用 `determine_stage()`（`src/sdd/spec/validation.rs:1583`）计算 stage
  - 新增字段 `stage`（draft/specified/designed/full）、`artifacts`（已存在 artifact 文件名列表）、`readyToImplement`（stage==full）
  - text 模式：在标题行后打印 `Stage: <stage>`
  - 验证：`just run sdd show <a-full-change> --json` 含三新字段；draft change 的 `readyToImplement==false`

- [ ] **R2: validate 非 strict 暴露 stage INFO（修 r45 偏差）**
  - 定位 `print_single_report`（`src/sdd/shared/validate.rs:520`）：valid 时直接 return 吞掉 INFO
  - 修改：valid 且存在 stage/非 full 提示时，打印精简阶段行（不打 ERROR/WARNING 块）
  - 去噪：非 full 阶段下不重复叠加 `tasks_missing`（draft 本就无 tasks，该提示对 draft 是噪音）
  - 验证：draft change 非 strict validate 输出可见 INFO 阶段提示

- [ ] **R3: apply skill 阶段守卫**
  - 编辑 `templates/sdd/en/skills/llman-sdd-apply.md`（及 zh-Hans 若存在）
  - 前置检查升级：`stage=$(llman sdd show <id> --json | jq -r .stage)`；非 full → STOP + 引导 continue
  - draft 文案明确："这是 draft 提案，需先补 specs → design → tasks 长大成 full"
  - 验证：模板含守卫文本；`just check-sdd-templates` 通过

- [ ] **R4: verify skill 阶段守卫**
  - 编辑 `templates/sdd/en/skills/llman-sdd-verify.md`：非 full → STOP + 引导 continue
  - 验证：模板含守卫文本

- [ ] **R5: continue skill 反向感知 draft**
  - 编辑 `templates/sdd/en/skills/llman-sdd-continue.md`：draft 阶段显式提示"draft 提案，需先补 specs → design → tasks 长大到 full 方可实现"
  - 验证：模板含 draft 提示

- [ ] **R6: 集成测试**
  - 新增 `tests/sdd_show_stage_tests.rs`（或追加现有）：draft/full change 的 show --json 断言 stage/artifacts/readyToImplement
  - 追加 validate 非 strict draft 打印 stage INFO 的断言
  - 全程用 `tempfile::TempDir` + `TestProcess`，不污染 repo
  - 验证：`just test` 通过

- [ ] **R7: 质量门禁**
  - `just fmt`
  - `just lint`
  - `just check-sdd-templates`
  - `just test`
