## 1. Implementation
- [x] 1.1 增加 skills 根目录解析（CLI flag + ENV + config.yaml + 默认路径）。
- [x] 1.2 扩展 llman config 结构（新增 `skills.dir`）并保证缺省解析兼容（本地 `.llman/config.yaml` 优先于全局）。
- [x] 1.3 调整 `SkillsPaths::resolve()` 与 skills 命令入口以使用新的根目录。
- [x] 1.4 更新/新增错误信息与帮助文案。
- [x] 1.5 补充测试：路径解析优先级、缺省回退、env/cli 覆盖。

## 2. Verification
- [x] 2.1 `cargo +nightly test --all`
- [x] 2.2 手动 smoke: `llman skills`（默认）、`llman skills --skills-dir /tmp/llman.skills`
