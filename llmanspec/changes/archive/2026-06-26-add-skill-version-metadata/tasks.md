# Tasks: add-skill-version-metadata

## 实施任务

- [x] 1. 修改 skill 模板，增加 `metadata.version` 字段占位符
  - 文件：`templates/sdd/*/skills/*.md`
  - 验证：模板包含 `metadata:` 和 `version:` 字段

- [x] 2. 修改 `llman sdd init` 命令，生成 skills 时填充当前 CLI 版本
  - 文件：`src/sdd/project/templates.rs`
  - 验证：`build_template_vars` 函数添加 `llman_version` 变量，模板渲染后包含 `metadata.version: "0.0.50"`

- [x] 3. 修改 `llman sdd update-skills` 命令，更新时同步版本
  - 文件：`src/sdd/project/update_skills.rs`
  - 验证：`update_skills` 使用 `skill_templates` 函数，自动填充版本

- [x] 4. 实现版本不匹配检查逻辑
  - 文件：`src/skills/catalog/scan.rs`
  - 验证：`check_skill_version_compat` 函数可检查版本兼容性

- [x] 5. 更新 `skills-management` spec，记录版本元数据要求
  - 文件：`llmanspec/specs/skills-management/spec.toon`
  - 验证：delta spec 已包含 r18 (技能版本元数据) 和 r19 (版本不匹配警告)

- [x] 6. 添加单元测试
  - 文件：`src/skills/catalog/scan.rs`
  - 验证：`cargo test` 通过，包含 7 个新测试用例

## 校验命令

```bash
# 格式检查
cargo +nightly fmt -- --check

# lint 检查
cargo +nightly clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo +nightly test --all

# 校验 spec
llman sdd validate add-skill-version-metadata --strict --no-interactive
```
