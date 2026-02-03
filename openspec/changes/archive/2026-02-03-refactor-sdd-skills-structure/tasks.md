## 1. 规划与准备
- [x] 1.1 确认目标目录结构与模块映射表（sdd/skills），列出必须保留的公开接口（不做兼容性 re-export）
- [x] 1.2 确认 `include_str!` 更新策略（相对路径修正或 `CARGO_MANIFEST_DIR` 拼接）

## 2. SDD 结构重组
- [x] 2.1 创建 `src/sdd/{change,spec,project,shared}` 目录并迁移对应文件
- [x] 2.2 调整 `src/sdd/mod.rs` 与 `src/sdd/command.rs` 的模块声明与 re-export
- [x] 2.3 若需要，抽取 show/validate 的共享 item 解析逻辑到 `shared/item.rs`
- [x] 2.4 修正 `templates.rs` 的 `include_str!` 路径以匹配新位置

## 3. Skills 结构重组
- [x] 3.1 创建 `src/skills/{cli,catalog,targets,config,shared}` 目录并迁移对应文件
- [x] 3.2 调整 `src/skills/mod.rs` 的模块声明与 re-export，保证 `crate::skills::*` 对外路径稳定
- [x] 3.3 更新 `src/prompt.rs` 对 `find_git_root` 的引用（或保留旧路径代理）

## 4. 验证与回归
- [x] 4.1 `cargo +nightly test sdd_integration_tests`
- [x] 4.2 `cargo +nightly test skills_integration_tests`
- [x] 4.3 `just check-all`
- [x] 4.4 `justfile` 添加 `alias qa := check-all`
