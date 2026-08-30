# Tasks: remove-sdd-skill-set-meta

- [x] T1 模板与渲染链
  36 处模板删除 `metadata.llman_sdd` 子块；`load_skill_template` 的 skill_set 参数与 `{{ skill_set }}` 变量退役；resync；模板门禁绿。
- [x] T2 门禁瘦身
  skill_consistency 重写为 jinja-only 卫生检查（元数据解析/校验删除）；调用点同步；单测重写。
  [blocked-by: T1]
- [x] T3 测试与 locales
  bdd_steps/it fixture 去除 metadata 写入；i18n `skill_set_invalid` 键删除；全测试绿。
  [blocked-by: T2]
- [x] T4 全量门禁
  fmt/clippy/测试（含 BDD）/validate --all/review；建议 llman-sdd-verify。
  [blocked-by: T1, T2, T3]
