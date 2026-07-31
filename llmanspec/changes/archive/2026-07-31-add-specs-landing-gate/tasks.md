# Tasks

- [x] T1 合约：扩展 `sdd-workflow` requirement（specs landing + readyToImplement 修订）与 `.feature` 可执行场景；更新 r93 场景期望；r124 纳入 `skip_specs_landing`
- [x] T2 CLI 核心：`specs_landed` / `skip_specs_landing` 探测（`base...branch` × `llmanspec/specs`）+ `readyToImplement` 收紧；`show --json` / `status` 暴露字段 [blocked-by: T1]
- [x] T3 validate WARNING + 友好错误文案（含 skill 名引导）；默认分支脏 specs 提示 [blocked-by: T2]
- [x] T4 文档/skill：根 `AGENTS.md` 流水线补 Specs landing；纠正 `llman-sdd-propose`（templates + `.agents`）先 start 再改 specs；`llman-sdd-apply` 读 ready/specsLanded 并 STOP [blocked-by: T1]
- [x] T5 BDD/集成：扩展 `bdd_steps` fixture（可选 specs commit / skip 旗标）；`just test` 相关 + `llman sdd validate add-specs-landing-gate --strict --no-interactive` [blocked-by: T2, T3, T4]
