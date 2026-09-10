---
depends_on: []
branch: sdd/specs-flat-layout
base_sha: 0189a539a6da68dc280029bdf85862a320b63e6e
checkpointed: true
rules_edit_acked: true
checkpoint_sha: 0189a539a6da68dc280029bdf85862a320b63e6e
---

# Specs 布局扁平化 + 平铺迁移 + 协作提示

> 本草案只定义**需求与设计**，不包含实现——实现者按 `design.md`（含完整现状调研与决策记录）+ `tasks.md`（垂直切片）落地。
> 停靠在 Designed（未 `change start`/`attach`）；Specs landing 与实现阶段的 live `.feature` 编辑由实现者在其绑定分支上完成。
>
> 与备选方案的关系：本提案选「双布局兼容 + migrate 一次性平铺」路径（而非「强制纯扁平零兼容」），原因是存量项目不受迫迁移、解析规则收口成一个单源，未来收紧成纯扁平也只是删一个分支的小改。相关取舍记录见 `design.md` §决策记录。

## Why

现状 `llmanspec/specs/**` 强制「目录化」：每个 capability 是 `specs/<cap>/<cap>.feature` 单文件目录，r131 更进一步要求目录内**恰好一个** `.feature`（多文件 = ERROR）。这层目录是纯负担：

- **同一身份三重冗余**：目录名、文件名、`# capability:` 头注释三者被 validate 强制一致（`validation.rs` 的 War "must match spec directory name"）。
- **目录没有任何「组」语义**：本仓库 20+ 个 capability 目录全部「每目录恰 1 文件且同名」；没有任何读取方依赖目录装别的东西（r131 禁止第二 `.feature`，也没有方读附属文件）。按 arch-review 的「删除验证」：把 `specs/<cap>/<cap>.feature` 压平为 `specs/<cap>.feature`，复杂度不会在别处重新冒出来。
- **对 agent 与人的操作成本**：`ls specs/` 无法一览能力清单；agent 路径多一层 token；每个新 capability 一次多余 `mkdir`。
- **强制性 ERROR 会卡用户主线**：用户整理期在目录里并排几个草稿 `.feature` 再清理，会被「目录内多 .feature = ERROR」强迫立即 migrate/提级。

## What Changes

1. **布局二选一（改写 spec-format r131 条文，保留编号）**
   - 扁平 `llmanspec/specs/<cap>.feature` —— 新默认；cap id = 文件 stem。
   - 目录 `llmanspec/specs/<cap>/` —— 保留兼容；cap id = 目录名；目录内可多个 `.feature`（主文件 = 同名文件优先；多文件从 ERROR 放宽为 Warning，不阻断任何操作）。
   - 同一 id 两种布局并存 → 冲突 ERROR（保证 `resolve` 确定性）。命名约定（文件名 vs 目录名 vs `# capability:`）由项目自己约定，CLI 不强制；header 与 id 不一致保留现状 Warning。
2. **`llman sdd project migrate --kind specs-flatten`（新增 kind，toon2features 保留）**
   - 只处理「纯同名单文件目录」`specs/<cap>/<cap>.feature`：`git mv` 平铺 + 删空目录，git 保留历史。
   - 预检查 5 类**只报告不处理**：目标扁平路径重名冲突、目录含 `spec.toon`（先 toon2features）、目录含多个 `.feature`、目录含非 `.feature` 附属文件、文件名 ≠ 目录名。
   - 目录 scope 自引用自动改写（`# scope: llmanspec/specs/<cap>` → `llmanspec/specs/<cap>.feature`）；`--dry-run` 预览；幂等。
3. **协作提示固化到 CLI**
   - `project migrate` 执行输出注入「scope 检查」提示块（scope 职责解释 + 建议指向真实源码目录 + 下一步 `validate --specs`）。
   - `project migrate --prompt` 打印内置协作说明（命令意图 / agent 与人类分别做什么 / 陷阱 / 下一步命令），模板托管在 sdd templates units 编译期嵌入——先落地 migrate 相关命令，机制可扩展。

详见 `design.md`（现状影响面逐文件、SpecLoc 解析设计、预检查清单、r131 新条文草案、`--prompt` 机制）。
