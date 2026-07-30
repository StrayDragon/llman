# Design: add-proposal-frontmatter-schema-guard

## 关键设计决策

### D1: 未知字段报 ERROR 而非 WARNING

**决策**：合法字段集外的键报 `ValidationLevel::Error`。

**理由**：`status` 等伪字段此前靠范例模仿扩散成伪惯例，WARNING 不足以阻断——agent 容易忽略 WARNING 继续。ERROR 强制清理，从源头让伪惯例不可存活。已与用户确认走 ERROR 路线。

**权衡**：已 attach 的 active change 若含历史遗留伪字段会首次报错。这是预期内的破坏性变更（行为合约变更），清理即可。archived 免检（D2）保证历史归档不受影响。

### D2: archived changes 免检

**决策**：`changes/archive/` 下的 proposal 不受未知字段校验约束。

**理由**：archived 是只读历史记录，零迁移成本，不改动历史归档。校验仅作用于 active changes。已与用户确认。

**实现**：校验入口遍历 changes 时，对路径含 `changes/archive/` 的 proposal 跳过未知字段检测（其他 frontmatter 校验如 depends_on 仍保留现有行为）。

### D3: 合法字段集来源

**决策**：合法字段集 = `check_proposal_frontmatter` 当前已识别的全部键。

**字段集**：`depends_on`、`blocks`、`branch`、`base_sha`（含 `baseSha` 别名）、`checkpointed`、`checkpoint_sha`（含 `checkpointSha` 别名）。

**理由**：这些是 CLI 已消费的字段（attach binding、checkpoint、依赖图）。不另发明新字段集，直接固化现有已识别集合，避免引入"校验规则"与"解析代码"的双 SSOT。

### D4: stage 仍是推断量

**决策**：`determine_stage` 行为完全不变，继续从磁盘 artifacts + attach binding 推断（r93 三态）。MUST NOT 引入任何 frontmatter 字段（含 status）影响 stage。

**理由**：stage 已是 CLI 完整暴露的推断量（`status`/`show` 命令输出）。把 status 作为存储字段会与 determine_stage 形成双 SSOT。废弃 status、只用推断的 stage 是单一权威。

## 不做的事

- 不删除任何现有合法字段的读取（只新增未知键检测）。
- 不写迁移脚本清理 archived（D2 免检）。
- 不改 `change new` 骨架（当前骨架已无 status，符合目标）。
