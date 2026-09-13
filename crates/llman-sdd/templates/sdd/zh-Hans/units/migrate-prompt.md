# llman sdd project migrate — 协作说明

## 命令意图

- `llman sdd project migrate --kind toon2features`：遗留 `spec.toon` → 单轨 `.feature`（一次性迁移，幂等）。
- `llman sdd project migrate --kind specs-flatten`：纯同名单文件目录 `specs/<cap>/<cap>.feature` → 扁平 `specs/<cap>.feature`（git mv 保留历史）。

## Agent 该做什么

- 先确认是否真需要迁移（无 legacy / 单文件目录 → no-op）。
- 先跑 `--dry-run` 看预检查报告；`conflict` / `misnamed` 项人工处理，勿强行迁移。
- 迁移后运行 `llman sdd validate --specs --strict --no-interactive` 与项目 BDD 套件。

## 人类该做什么

- 检查 `scope_rewritten` 报告与 git diff（mv 保留历史）。
- 顺手把 `# scope:` 指向该规范管辖的真实源码目录。

## 陷阱

- 含多个 `.feature`、异名文件或附属文件的目录不会自动扁平（只报告）。
- 重名冲突必须人工解决（两文件都保留）。
- 自引用 `# scope:` 会被自动改写为 `specs/<cap>.feature`。

## 下一步

- `llman sdd validate --specs --strict --no-interactive`
- 项目 BDD 运行（如 `cargo test --features bdd`）
