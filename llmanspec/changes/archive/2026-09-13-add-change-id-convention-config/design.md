# Design: add-change-id-convention-config

三个需要定案的权衡点。均已选定方向；apply 时若发现新事实导致方向不成立，回 explore/propose 重新裁决，不 ad-hoc 改。

## D1: 冻结包（7z）扫描——best-effort，不引入硬依赖

**问题**：xylitol 实测撞号的真正大头在 `changes/archive/freezed_changes.7z.archived`（包内 415 个 change，最大 c2790）。但 llman 核心不应假设 `7z` 二进制存在（CI/下游环境不可控）。

**决策**：
- 取号扫描主体 = `llmanspec/` 全树递归的**目录名**（覆盖 `delayed-changes/` 递归、`changes/archive/` 首层与深层——这两类是纯文件系统扫描，零依赖）。
- 冻结包：扫描时遇到 `changes/**/*.7z` / `*.archived` / `*.zip` / `*.tar*` 等压缩包形态文件，若 `7z`（或 `unzip`/`tar`，按扩展名映射）在 PATH 上则 best-effort 列取包内 change-id 形态目录名；命令不存在或执行失败 → **不失败**，`next-id` 与 `change new --from`（渲染了 `llman_sdd_unique_id` 时）输出 WARNING 列出未能透视的冻结包路径，提示人工核对。
- 文档化约定写入 `--from` help 与 next-id 输出文案：冻结包内编号视为继续占用。

**否决项**：内置 7z 解析 crate（`sevenz-rust` 等）——为 best-effort 场景加编译依赖与攻击面不值；把冻结包扫描做成必选（失败即报错）——破坏「无 7z 环境可用」。

## D2: 号码提取算法——优先 pattern 命名捕获组，缺省内置启发式

**问题**：「change-id 形态的目录名」如何提取数字？配了 pattern 的仓库语义清晰；没配 pattern 只配了 template 的仓库呢？

**决策**：
- 配置了 `change_id.pattern` 且含命名捕获组 `(?P<unique>[0-9]+)` → 用它提取。
- 否则用内置启发式：目录名中首个 `c` 或 `change` 前缀紧跟的数字段（`c2790-…`、`2026-09-13-c20-…` 中的 `c20`），即 `[c](\d+)` 边界匹配；提取不到数字的目录名不参与取号（只参与 `llman_sdd_unique_id` 的占用判定不适用——该变量只由数字决定）。
- `llman_sdd_unique_id` = 全树提取到的最大号 + 步进 5？——**否**：+1。issue 实测里 xylitol 人工跳到 c2795/c2800 留了余量，但工具语义保持最小正确（+1）；留余量是用户策略，交给 template 写死偏移或人工 `--verb`/显式 id 承担，不进工具默认。全树无任何可提取号码的目录时首号从 1 开始（dry-run 验收场景依赖此约定：`c1-…`）。
- 纯目录名扫描的「占用」判定只服务取号（数字提取），不做字符串级 id 唯一性检查（那是 r127 发现阶段已有职责，保持不动）。

## D3: pattern 校验的范围与时机——active 发现产物，validate 单点

**问题**：pattern 校验放 discovery 层（发现即拒绝）还是 validate 层？

**决策**：validate 层。理由：
- discovery（r127）职责是结构唯一性与路径合法性，是把 change 变成「可操作对象」的前置；pattern 是项目风格规约，违反时 change 依然必须可 list/show/diff（用户要能看见它、修它）。discovery 拒绝会导致撞 pattern 的 change 直接消失于工具视野，不可修。
- 校验点 = validate 对 active changes 的既有遍历（`collect_change_issues_fast` / `validate_change_full` 一侧），每违规 id 一条独立 ERROR，消息含 id 全文与 pattern 原文。
- archive/ 免检沿用 r124 既有豁免；`delayed-changes/` 等 llman 视野外的目录本来就不进 active 集合，天然不校验——「存量不回溯」无需额外代码。

**否决项**：`change new` 创建时即拒（不配 pattern 的仓库无此 gate；配了 pattern 的仓库经 `--from` 模板生成的 id 天然合规，显式 `<CHANGE>` 传参违规时应允许创建但在 validate 报错——与「specs 可以先写错再 validate 修」的心智一致）。

## 变量命名与渲染细节（非争议记录）

- 预设变量 `llman_sdd_` 前缀：grep 友好、避免撞用户变量；minijinja 渲染用 `UndefinedBehavior::Strict`（llman-sdd `templates.rs` 已有先例），模板引用未注入变量即报错——非法模板变量在 validate/config 加载期给出清晰报错（issue 验收项）。
- `date` 变量 = 本地日期 `%Y-%m-%d`（对齐 archive 日期前缀形态）。
- `verb` 归一词表：`add/update/remove/refactor/fix`；`--from` 描述首词命中词表则取用，否则 `--verb` 显式指定；两者皆无 → 模板引用 `verb` 时报错提示补 `--verb`。
- `subject` = 现 `derive_change_id` 去除 verb 前缀后的剩余产物（避免模板拼出 `fix-fix-…` 重复）；无剩余时回退完整派生产物。
