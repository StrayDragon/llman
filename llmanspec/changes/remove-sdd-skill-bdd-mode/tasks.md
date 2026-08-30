# Tasks: remove-sdd-skill-bdd-mode

- T1 模板与渲染变量
  36 处模板 frontmatter 删 `bdd_mode: "{{ bdd_mode }}"`（双 locale）；build_template_vars 移除 bdd_mode；resync；模板门禁绿。
- T2 门禁机器退役
  skill_consistency 移除 bdd_mode 解析/比对（保留 llman_sdd/skill_set 门禁）；validate 路径同步；单测适配。
  [blocked-by: T1]
- T3 测试适配
  bdd_steps fixture（given_skill_dir/global_skills_config/seed 链路）与 it 测试中 bdd_mode 断言删除或改写；r95 两个 @executable 场景经新 fixture 驱动全绿。
  [blocked-by: T2]
- T4 全量门禁
  fmt/clippy/测试（含 BDD）/validate --all/review；建议 llman-sdd-verify。
  [blocked-by: T1, T2, T3]
