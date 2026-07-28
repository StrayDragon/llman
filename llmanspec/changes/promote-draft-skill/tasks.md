# Tasks: promote-draft-skill

## 垂直切片 1：Rust 常量与技能注册（draft 提升为默认）

- [x] 1.1 在 `src/sdd/project/templates.rs` 把 `llman-sdd-new-change.md` 从 `OPTIONAL_SKILL_FILES` 移到 `DEFAULT_SKILL_FILES`，并改名为 `llman-sdd-draft.md`
  - 验证：`cargo check` 通过
- [x] 1.2 在 `src/sdd/project/config.rs` 把 `OPTIONAL_SKILL_NAMES` 中的 `llman-sdd-new-change` 移除（draft 已是默认，不在 optional 列表）
  - 验证：`cargo check` 通过
- [x] 1.3 在 `src/sdd/project/config_skills.rs` 的 `skill_description()` 移除 `llman-sdd-new-change` 条目（draft 不再需要 optional 描述）
  - 验证：`cargo check` 通过
- [x] 1.4 在 `src/sdd/project/config.rs` 的 `DEFAULT_CONFIG_EN` / `DEFAULT_CONFIG_ZH` 注释示例里把 `llman-sdd-new-change` 替换为其它仍 optional 的 skill（避免注释里出现已不存在的项）
  - 验证：`cargo check` 通过

## 垂直切片 2：模板文件重命名与重写

- [x] 2.1 删除 `templates/sdd/en/skills/llman-sdd-new-change.md` 与 `templates/sdd/zh-Hans/skills/llman-sdd-new-change.md`
  (blocked-by: 1.1]
- [x] 2.2 新建 `templates/sdd/en/skills/llman-sdd-draft.md`：职责单一化为 draft shell only（复用 `change new --from`），含 ethics 治理字段、Pipeline 位置图、指向 propose 的升级引导
  - 验证：模板含 `ethics.risk_level` 等 5 个必需 key（经 structured-protocol unit 注入）
- [x] 2.3 新建 `templates/sdd/zh-Hans/skills/llman-sdd-draft.md`：与 en 对等的中文版
  (blocked-by: 2.2]
  - 验证：`just check-sdd-templates`（locale parity）通过

## 垂直切片 3：propose 模板裁剪

- [x] 3.1 在 `templates/sdd/zh-Hans/skills/llman-sdd-propose.md` 裁剪「轻量 draft 路径（仅 draft proposal）」整段，替换为一句引导：「仅记草案用 `llman-sdd-draft`；本技能专注完整提案」
  - 验证：渲染产物不含完整 draft 路径步骤
- [x] 3.2 在 `templates/sdd/en/skills/llman-sdd-propose.md` 同步裁剪对应段（若存在）
  (blocked-by: 3.1]
  - 验证：`just check-sdd-templates` 通过

## 垂直切片 4：embedded_template match arm 重命名

- [x] 4.1 在 `src/sdd/project/templates.rs` 的 `embedded_template()` 把 4 个 `llman-sdd-new-change.md` match arm（en + zh-Hans 各路径）改为 `llman-sdd-draft.md`
  (blocked-by: 2.2, 2.3]
  - 验证：`cargo build` 通过（include_str! 路径存在）

## 垂直切片 5：测试更新

- [x] 5.1 更新 `src/sdd/project/update_skills.rs` 中断言 `llman-sdd-new-change` 不默认写入的测试，改为断言 `llman-sdd-draft` 默认写入、`llman-sdd-new-change` 不写入
  (blocked-by: 1.1, 4.1]
  - 验证：`cargo nextest run -p llman --lib sdd::project` 通过（48 passed）
- [x] 5.2 更新 `tests/sdd_bdd_compat_tests.rs` 的 smoke 命令列表（若引用 new-change）
  - 验证：`cargo nextest run --test sdd_bdd_compat_tests` 通过（7 passed）；额外修了 `tests/sdd_integration_tests.rs::test_sdd_config_skills_non_interactive`（available 数量 7→6）

## 垂直切片 6：文档与门禁

- [x] 6.1 更新 `AGENTS.md` 可选增强能力表中的 skill 列表（若有 new-change 引用）
  - 验证：AGENTS.md/docs/ 无 new-change 引用，无需改动
- [x] 6.2 运行 `just check`（fmt + lint + test）全绿
  (blocked-by: 5.1, 5.2]
  - 验证：502 tests passed
- [x] 6.3 运行 `just check-sdd-templates`（版本头 + locale parity）全绿
  (blocked-by: 2.3, 3.2]
  - 验证：SDD template checks passed for locales: en, zh-Hans
