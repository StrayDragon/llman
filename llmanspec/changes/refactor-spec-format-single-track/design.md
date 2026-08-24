# Design: 单轨 feature-as-spec 格式重构

## 目标态

```
llmanspec/specs/<cap>/           # 每 capability 一个目录一个 .feature
└── <cap>.feature                # 唯一规格载体
    # capability: <name>         ← 元数据头注释（staleness/context 消费）
    # purpose: ...
    # scope: src/...

    @req:r57 @human              ← 约束层（statement 全文在 description，无损）
    场景: <title>
      [GWT 鼓励性分解；空 When 仅 WARNING]

    @req:r57 @executable         ← 验收层（runner 展开，现有内容零迁移）
    场景: <scenario.id>
      假如/当/那么 …（复用泛化 step 库）
```

## 关键决策

### D1 单轨 tag 治理，而非 YAML rules 层 / Rule 块

- rstest-bdd-macros 0.6.0-beta3 `scenarios!` 只遍历顶层 `feature.scenarios`，`Rule:` 块内场景被**静默跳过**（源码验证）；内部解析器硬编码 `rules: Vec::new()`。→ 可执行场景 MUST 留在顶层。
- YAML rules 层被否决：两种格式重回双轨；GWT 三槽位本身就是「强迫分解散文」的程序化约束手段。

### D2 IR 稳定，backend 替换

`MainSpecDoc` 结构保留；新增 `FeatureBackend`（实现现有 `SpecBackend` trait）从 tag 场景 + 头注释填充。`list --specs`/`show`/`context`/`index rebuild` 输出形状基本稳定，仅 morphology 换血。

### D3 三态 morphology（取代 harness bound/unbound）

| 计数 | 定义 |
|---|---|
| `rule_count` | `@human` 且带 `@req` 的去重 id 数 |
| `rule_enforced_count` | 有 ≥1 个 `@executable` 实现的 rule |
| `rule_manual_count` | 显式 `@manual` 豁免的 rule |
| `rule_pending_count` | 其余（覆盖缺口） |
| `acceptance_count` | `@executable` 场景总数 |
| `orphan_acceptance_count` | 无 `@req` 的 `@executable`（WARNING） |

`dual_write_count` 删除。`bdd.bindings` 配置退役（tag 即声明；保留可选 override 以兼容下游自定义 step 库 tag）。

### D4 锁定哈希与解锁

- 规范化输入：scenario id + name + description + steps 文本，逐行 trim 尾随空白后拼接 SHA-256；不做关键词翻译归一（文件头 `# language:` 固定，跨语种漂移视为改动）。
- 门禁挂点：`validate --strict` 与 `change finalize`/`checkpoint`；对比 `base_sha...HEAD` 中 `llmanspec/specs/**/*.feature` 内 `@human` 场景哈希集合。
- 任何增删改 → ERROR；豁免通道：proposal frontmatter 新增合法字段 **`rules_edit_acked: true`**（须同步扩展 r124 合法字段集与 schema）。

### D5 迁移（toon2features）

`project migrate --kind toon2features`：requirements[] → `@req:<id> @human` 场景（statement 全文入 description）；`scenarios[feature:false]` note 行丢弃；同目录既有 `.feature` 内容合并进同一文件；幂等；迁移报告列出三态初值。自迁移在本 change 分支上完成（apply 阶段 t6）。

### D6 零兼容

引擎切换 commit 后 validate 遇遗留 `spec.toon` → ERROR 并指向 toon2features；`migrate --kind spec-md2toon`、partitioned 门禁、dual-write 同批删除（沿用 solidify 先例）。下游仓库靠随版本发布的迁移 prompt。

### D7 本 change 自身的 Specs landing 用旧格式书写

Branch binding 后、引擎落地前，live specs 仍以 toon+feature 表达目标合约（SSOT 纪律不破例）；apply 的自迁移任务把全库切到新格式。这是刻意的中间态。

## 兼容性矩阵

| 面 | 策略 |
|---|---|
| `.feature` 可执行层 | 零迁移（88 个文件、tag、泛化 step 库不动） |
| change 生命周期 | 不动；specsLanding glob 收窄 `*.feature` |
| `validate --check` / `bdd:` | 语义不变（纯 runner 开关；无 bdd 段项目 fast mode 结构校验继续可用） |
| BDD-off 下游项目 | 一次性迁移 prompt；fast mode 兜底 |
| 术语债 | 「harness bound/unbound」→ rule 三态口径，文档渐进替换 |

## 风险

- R1 锁定哈希假阳性（空白/换行差异）：规范化规则单测覆盖；首版保守（宁报错勿漏报）。
- R2 大爆炸自迁移规模：28 spec + compat tests + templates/skills 同分支；tasks 按垂直切片排依赖，finalize 前 `just check-all` 全绿为准出。
- R3 rstest-bdd 上游演进（未来支持 Rule）：本设计不被阻塞，届时可作后续增强。
