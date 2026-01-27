## 1. Locale 配置与模板加载
- [x] 1.1 新增 `llmanspec/config.yaml` 读写（version/locale/skills 路径），默认 locale 为 `en`
- [x] 1.2 实现 sdd 模板的 locale 解析与回退链（`zh-Hans` → `zh` → `en`）
- [x] 1.3 调整模板目录为 `templates/sdd/<locale>/...`，补齐 en / zh-Hans 版本（AGENTS、root stub、project、spec-driven、skills）

## 2. init/update 行为更新
- [x] 2.1 `llman sdd init --lang <locale>` 写入 config 并用 locale 渲染 `llmanspec/AGENTS.md` 与模板
- [x] 2.2 `llman sdd init/update` 生成或刷新 root `AGENTS.md` 受管块，保留非受管内容
- [x] 2.3 `llman sdd update` 基于 config locale 刷新模板，不修改 `specs/` 与 `changes/`

## 3. update-skills 子命令
- [x] 3.1 新增 `llman sdd update-skills`（交互选择 Claude/Codex，并允许输入输出路径）
- [x] 3.2 支持非交互 `--all`、`--tool claude,codex` 与可选 `--path` / `--no-interactive`
- [x] 3.3 生成/刷新 `<tool>/skills/<skill>/SKILL.md`（覆盖托管内容，不生成 slash commands）
- [x] 3.4 支持模板内 `{{region: <path>#<name>}}` 引用并解析对应 region 块

## 4. 校验提示与 CLI 文案
- [x] 4.1 在 validate 输出中补充缺段落/缺描述/场景格式错误/无 delta 的修复提示与最小示例
- [x] 4.2 补齐 `locales/app.yml` 的 `sdd.*` 英文词条

## 5. 文档与验证
- [x] 5.1 更新 `README.md`：新增 `llman sdd update-skills` 与 `llmanspec/config.yaml` 说明
- [x] 5.2 添加 locale 配置解析与 update-skills 路径选择的测试
- [x] 5.3 运行 `just test`（或 `cargo +nightly test --all`）并使用 `LLMAN_CONFIG_DIR=./artifacts/testing_config_home`
