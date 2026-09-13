# Tasks: add-change-id-convention-config

## 测试边界（seam，写码前确认）

全部复用既有 harness seam，不发明新 seam：

1. **CLI 子进程**（`tests/it/` 集成 + BDD step）：`llman sdd validate --all --strict`、`llman sdd change new --from [--dry-run] [--verb]`、`llman sdd change next-id`、config 加载错误路径。
2. **公共函数**：`derive_change_id`（subject 语义调整的单测面）、全树取号 helper（纯函数单测：目录树 fixture + 号码提取）。
3. **BDD**：`tests/bdd_steps.rs` 既有通用 step 驱动新增 `@executable` 场景（rstest-bdd，`cargo test --features bdd`）。

## Tasks

- [ ] T1 config `change_id:` 段（垂直切片：schema → 解析 → 测试）
  新增 `ChangeIdConfig { pattern: Option<String>, template: Option<String> }` 于 `sdd/project/config.rs`（schemars derive + `skip_serializing_if`，与 `archive:`/`bdd:` 平级挂到 `SddConfig`）；非法 pattern 在加载/使用期编译检查并清晰报错；刷新 config schema 产物。
  验证：单测（解析接受/拒绝、缺省 None）+ `llman self schema check`。

- [ ] T2 全树取号 helper（纯新能力，无前置）
  `llmanspec/` 全树递归收集 change-id 形态目录名并提取号码：pattern `(?P<unique>[0-9]+)` 优先，缺省 `[c](\d+)` 边界启发式（design D2）；压缩包形态 best-effort 透视（design D1，工具缺失/失败不报错）；返回占用号集合与最大号。
  验证：单元测试（tempfile 目录树 fixture：`changes/`、`changes/archive/`、`delayed-changes/tools/` 深层、无号码目录、7z 场景 mock/跳过）。

- [ ] T3 validate pattern 门禁 [blocked-by: T1]
  validate 对 active change id 做 pattern full-match，违规 = 独立 ERROR（含 id 全文与 pattern 原文）；未配置 `change_id.pattern` = 零新失败类别（r265 既有约束措辞对齐）。archive/ 免检沿用 r124 豁免。
  验证：`tests/it/` 集成（配 pattern 违规报 ERROR 退出非零 / 未配置行为与现状逐字节一致）。

- [ ] T4 `change new --from` 模板渲染 + `--dry-run` [blocked-by: T1, T2]
  配置 `template` 时渲染生成 id（变量 `llman_sdd_unique_id`/`verb`/`subject`/`date`，Strict undefined；`--verb` 显式指定；subject = derive 产物去 verb 前缀，design D3 记录）；`--dry-run` 只打印不落盘；未配置 = 现状启发式不变；`--from` help 文本诚实化（配置 template 后「follows naming conventions」为真，未配置描述启发式现状）。
  验证：单测（渲染变量各分支、subject 去 verb、CJK 描述）+ `tests/it/`（dry-run 只读断言：无目录创建；模板产出过 pattern）。

- [ ] T5 `change next-id` 子命令 [blocked-by: T2]
  挂 `SddChangeCommands`，只读打印全树最大号与下一可用号（`llman_sdd_unique_id` 取值依据）；冻结包不可透视时输出 WARNING；`--json` 形态（对齐既有命令风格）。
  验证：`tests/it/`（只读断言 + 构造「active 最大号 < delayed 最大号」fixture，取号不冲突）。

- [ ] T6 BDD 场景绑定与测试收口 [blocked-by: T3, T4, T5]
  `tests/bdd_steps.rs` 通用 step 驱动 sdd-workflow 新增 `@executable` 场景；`tests/it/main.rs` 注册新模块；`just test` 全绿 + `validate --all --strict` 全绿。
  验证：`cargo test --features bdd`（bdd.bindings run_command）+ `just check`。
