# Tasks: Change 名前缀匹配

## Seam 确认

本次变更涉及以下 seam（均为 CLI 子进程边界，可复用 `tests/bdd_steps.rs` 现有 harness）：

1. `llman sdd show <change>` — 验证前缀匹配 resolve
2. `llman sdd validate <change>` — 验证前缀匹配 resolve
3. `llman sdd status <target>` — 验证前缀匹配 resolve（已有 `.feature` 覆盖）
4. `llman sdd graph <change>` — 验证前缀匹配 resolve
5. `llman sdd change archive <change>` — 验证前缀匹配 resolve
6. `llman sdd change attach <change>` — 验证前缀匹配 resolve（BDD-on only）
7. `llman sdd change checkpoint <change>` — 验证前缀匹配 resolve（BDD-on only）
8. `llman sdd change diff <change>` — 验证前缀匹配 resolve（BDD-on only）
9. `llman sdd change delta * <change_id>` — 验证前缀匹配 resolve
10. 精确匹配仍优先、多匹配报错、无匹配报错

---

## Tasks

### t1: 实现核心函数 `resolve_change_id`（含单元测试）

- [x] 实现完成

**文件**: `src/sdd/shared/discovery.rs`
- 新增 `resolve_change_id(root, input) -> Result<String>` 返回解析到的完整 change id 或错误
- 匹配逻辑：1) 精确活跃 → 2) 前缀活跃 → 3) 前缀归档 → 4) 无匹配报错
- 多匹配错误含完整候选列表

---

### t2: 更新 `show` + `validate` 使用 `resolve_change_id`

- [x] 实现完成

**文件**: 
- `src/sdd/shared/show.rs` — `show_direct` 中使用 prefix-aware resolution
- `src/sdd/shared/validate.rs` — `validate_direct` 中使用 prefix-aware resolution

---

### t3: 更新 `status` 使用前缀优先匹配

- [x] 实现完成

**文件**: `src/sdd/shared/status.rs`
- 重构 `resolve_target`：精确活跃 → 前缀活跃 → 前缀归档 → fallback fuzzy substring
- 保留优先级排序 + 归档检测等附属能力

---

### t4: 更新 `graph` 使用 `resolve_change_id`

- [x] 实现完成

**文件**: `src/sdd/shared/graph.rs`
- `build_seed_neighborhood` 先 resolve change id 再 BFS

---

### t5: 更新 change 子命令使用 `resolve_change_id`

- [x] 实现完成

**文件**: 
- `src/sdd/change/archive.rs` — `run_with_root`
- `src/sdd/change/git_native.rs` — `run_attach` / `run_checkpoint` / `run_diff`
- `src/sdd/change/finalize.rs` — `run_finalize`
- `src/sdd/authoring/delta.rs` — `run_skeleton` / `run_add_op` / `run_add_scenario`

---

### t6: 运行全量校验 + 测试

- [x] 编译通过
- [x] 全量测试通过（492/492，含 6 个新 prefix_resolve 单测，`just check` 全绿）
- [x] spec/feature 文档已完成（已在 propose 阶段完成）
- [x] 合约层单一权威收敛：删除 sdd-workflow/spec.toon 冗余 r111（与 cli/r112 描述同一合约，违反 req_id 全库唯一）；同步移除 interop-and-list-show.feature 中误标 @req:r111/r112 的前缀匹配 scenario（已由 cli/change-prefix-match.feature 覆盖）；清理 spec.toon 中 propose 阶段误带入的引号噪音
- [x] clippy 修复：discovery.rs / validate.rs 两处 collapsible-if（let-chains）

---

### t7: verify 驱动的合约/坏味修复

- [x] **C2（CRITICAL）**：删除 `status.rs::resolve_target` 的 substring `contains` fallback（违反 r112 MUST NOT）
- [x] **W1 + Shotgun Surgery**：抽取共享 `match_utils::prefix_resolve`（exact > prefix 单一真相源），`discovery::resolve_change_id` 与 `status::resolve_target` 均复用，消除平行实现与大小写语义分叉（统一为大小写敏感）
- [x] **W2**：`show.rs`/`validate.rs` 的 None 分支改为精确 spec 优先（避免 spec 名被 change 前缀劫持）
- [x] **坏味（Duplicated Code）**：`discovery.rs` 归档扫描段复用 `list_archived_changes`，消除 ~15 行重复
- [x] **SUGGESTION**：为 `prefix_resolve` 补 6 个单元测试（exact/unique-prefix/multi/none/case-sensitive/empty）
- [x] **SUGGESTION**：更新 design.md 反映实际实现（Result<String> 签名 + 共享 prefix_resolve + spec 优先规则）
- [x] **C1（声明）**：proposal.md 补「附带范围」段声明夹带的 `7eac336` dead-code 清理
