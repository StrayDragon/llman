---
depends_on: []
branch: sdd/improve-partitioned-ssot-agent-friction
base_sha: 97eca13f0f6a2c183d4d978b7a2bb56f6f4767a7
checkpointed: false
---

## Why

源自真实 agent 在 consumer 项目（xylitol）上跑 promote → apply → checkpoint → archive 闭环
后的操作性摩擦 field notes（`docs/release/partitioned-ssot/AGENT_FRICTION_PROMPT.md`）。
Skills 里的高层模型是对的（Partitioned SSOT + Git-native），但**操作性时序、失败归因、
flag 一致性**在实践中缺失，导致 agent 反复踩坑：

- dual-write 错误只说 `N executable scenario(s)`，不列具体 `(req_id, scenario.id)`，
  agent 无法直接定位去哪改；
- `change checkpoint` 缺 `--no-interactive`，而 archive / freeze / migrate 等同级 change
  子命令都有，agent 习惯性传该 flag 会撞「unknown argument」；
- skills 没有显式说明「哪些场景落 toon / 哪些落 .feature」的 2 列对照表，也没有
  checkpoint → commit metadata → archive → commit rename 的多阶段 commit 时序；
- agent 误以为 validate 的 Totals 行缺失失败 id（实际 FAIL 行已在上，但 grep view 会滤掉）。

## What Changes

**代码组（合约层，需同步 BDD 兼容测试）**：

- `validate` 的 dual-write 错误消息**必须列出**具体的 `(req_id, scenario.id)` 对，格式：
  `dual-write: N executable scenario(s) still have GWT in both spec.toon and .feature: [(r12, login-ok), (r15, logout)]; run llman sdd project migrate --kind partitioned`
- `change checkpoint` **必须接受并忽略** `--no-interactive` flag（对齐 archive/freeze/migrate，
  不实现真正的 interactive 模式——最小改动）。

**文档组（skills，非合约层）**：

- `propose` skill：加 Partitioned SSOT 双写形状 2 列对照表（Executable scenario vs Doc-only
  scenario 分别落 toon 还是 .feature）。
- `archive` skill：加 checkpoint → archive 的 3 阶段 commit 时序
  （commit live specs + code → checkpoint → commit checkpoint metadata → archive → commit archive rename）。
- `explore` / `verify` skill：加「诊断结构门禁优先 `validate <cap> --strict --no-check` 再 full」指引。
- `archive` skill：加「archived/frozen `depends_on` 被 validate 识别为 INFO（代码已正确处理
  `src/sdd/spec/validation.rs:1616-1621`），无需 archive 后手动改 frontmatter」。

## Capabilities

- `sdd-bdd-mode-compat`：增强 r6（dual-write 门禁消息含具体 id）+ r57（change 生命周期
  命令 flag 矩阵含 checkpoint --no-interactive）。
- `sdd-workflow`：checkpoint flag 补充（若 r57 不足以覆盖，则在此 capability 加 requirement）。

## Impact

- **合约层**：改 validate 输出语义 + change 命令 flag 矩阵 → 必须同步
  `llmanspec/specs/sdd-bdd-mode-compat/*.feature`（`sdd-bdd-mode-compat.feature` 的双写场景
  断言、`git-binding.feature` 的 checkpoint 场景）和
  `tests/sdd_bdd_compat_tests.rs`（smoke 列表）。
- **非合约层**：4 个 SKILL.md 文档更新，不影响 BDD 兼容测试。
- **非目标**：不改 validate 的 Totals 行格式（FAIL 行已在上，满足 prompt 精神）；
  不改 archived depends_on 解析逻辑（已正确处理，仅需文档化）。