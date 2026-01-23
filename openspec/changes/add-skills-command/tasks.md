## 1. 发现与存储
- [ ] 1.1 在 `LLMAN_CONFIG_DIR/skills` 下新增技能仓库路径与读写辅助
- [ ] 1.2 实现 `config.toml` schema v1 解析（sources/targets 列表，global/user）
- [ ] 1.3 实现 repo scope 自动发现（git root `.claude/skills`/`.codex/skills`），非 git 需确认
- [ ] 1.4 实现来源扫描与 `SKILL.md` 检测（递归 + `.gitignore`/全局忽略 + 跳过软链接）
- [ ] 1.5 实现 skill_id slugify 规则（frontmatter name -> slug，非法回退目录名）
- [ ] 1.6 实现目录哈希（md5）与快照元数据存储（排除忽略/软链接）

## 2. 导入与链接
- [ ] 2.1 实现导入（复制到仓库）与重链接（以软链接替换来源目录）
- [ ] 2.2 实现冲突检测与交互式选择流程
- [ ] 2.3 实现按 agent 目标目录启用/禁用操作

## 3. CLI 集成
- [ ] 3.1 在 CLI 中接入 `llman skills` 命令
- [ ] 3.2 实现交互界面与操作流程
- [ ] 3.3 新增本地化用户提示文案
- [ ] 3.4 实现非 git 目录扫描确认提示

## 4. 验证
- [ ] 4.1 新增 skill_id slugify 与回退规则的单元测试
- [ ] 4.2 新增 `.gitignore`/全局忽略 + 软链接跳过的单元测试
- [ ] 4.3 新增 repo 自动发现与非 git 确认流程测试
- [ ] 4.4 新增哈希、快照与冲突处理的单元测试
- [ ] 4.5 新增导入与软链接行为的集成测试
- [ ] 4.6 运行 `just test`（或 `cargo +nightly test --all`）并记录结果
