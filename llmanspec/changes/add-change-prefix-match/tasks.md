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
- [ ] 全量测试通过
- [ ] spec/feature 文档已完成（已在 propose 阶段完成）
