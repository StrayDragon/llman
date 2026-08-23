# Design: harness bound 可声明口径

## 1. 口径候选对比（探索结论）

| 方案 | bound 判定 | 配置成本 | 实测效果 |
|------|-----------|---------|---------|
| A. config 扫描规则 | glob+regex 提 `#[scenario]` (path,name) | 每项目配置 | xylitol 可用（格式规整）；对 llman 本仓无 per-scenario 绑定可扫 |
| B. manifest 约定 | 读约定文件 | 需生成/同步机制，否则 manifest 自身成为新谎言源 | 两仓都无现成生成机制 |
| C. 标签推导 | 场景 tags 含声明标签 | 零配置 | 本仓精确（497→50/447）；xylitol **全面反向失真**（0 标签 → 显示 0/293，真相 ~313/0） |

**决策：A + C 组合，作为可声明源；不设「聪明」默认。** 绑定真相在消费方代码里，llman 不扫描无从得知；任何固定默认都对某类项目说谎。未声明 = 保持现口径 count-all，零回归。

## 2. 默认行为（已决）

- 未声明 `bdd.bindings`：文本输出与今天逐字节一致；JSON 两新键为 `null`。
- 声明了源：bound/unbound 拆分生效，B+U MUST 等于 `harnessScenarioCount`。

理由：r39 已有「health/staleness 可为 null」先例；键恒存在 + null 值比键缺席对消费方更友好（schema 稳定，null-check 即可），且避免「未声明却显示 bound=0」的误导（xylitol 教训）。

## 3. 配置形状

```yaml
bdd:
  run_command: "cargo test --features bdd"
  bindings:
    - kind: tags
      tags: [executable]          # 场景须含全部所列 tag（@ 前缀可选）
    - kind: scenario-attrs
      files: ["tests/**/*.rs"]    # glob 相对仓库根；提取 path/name 字面量对
```

- 多个源之间**并集**；同一场景被多个源命中只计一次。
- `scenario-attrs.path` 解析出的 feature 路径按 `<cap>/` 段归属 capability；指向 specs 目录外的路径不计入任何 capability（Non-goal，见 proposal）。
- 校验：kind 未知 / tags 空 / files 空 → 配置解析报错（fail loudly，遵循项目错误处理原则）。

## 4. scenario-attrs 提取规则的表达力边界（关键取舍）

**做**：识别 `#[scenario( ... )]` 块内的 `path = "字面量"` 与 `name = "字面量"` 键值对，容忍多行与属性内空白（xylitol 实测 313 处全部此形态）。`index = N` 选择器命中的场景无法按 name 归属 → 该绑定忽略并计入解析警告（不失败）。

**不做**：开放 regex 由用户自定义提取逻辑。理由：正则写进 config 的维护成本和脆弱度高于收益；固定规则覆盖已知形态，遇到新形态再扩 kind（如未来上游 manifest）。

## 5. 数据流

```
config.yaml bdd.bindings ──► BindingSource 枚举（tags{tags} | ScenarioAttrs{globs}）
                                   │
            .feature 场景集 ◄──────┘ 绑定解析器 → HashSet<(feature_rel_path, scenario_name)>（tags 源直接按 tag 谓词命中）
                                   │
morphology 计算（partitioned.rs）：harness_scenario_count 不变；
  harness_bound_count = 命中数；harness_unbound_count = total − bound
```

- `Morphology` 新字段 `Option<usize>`：None=未声明源；Some=已声明。serde camelCase 序列化为 `harnessBoundCount`/`harnessUnboundCount`，旧 JSON 无此键仍可反序列化（向后兼容，sdd_bdd_compat_tests 断言）。
- 文本行：`Some((b,u))` 时输出 `harness-bound {b} harness-unbound {u}`；`None` 时维持 `harness {n}`。

## 6. 测试接缝（已确认）

fast mode（不标 `@executable`）：场景需「fixture 改写 config.yaml + 造带标签 `.feature`」组合步骤，泛化 step 库不具备（先例：nested-change-discovery.feature 注释）。实际断言由 Rust 测试驱动同一 CLI 边界；实现后可补 full mode step。

| Seam | 层 |
|------|---|
| CLI 子进程 `list --specs [--json]` / `show <spec>` | 集成测试（TestProcess + TempDir fixture） |
| `bdd.bindings` 反序列化 / 校验 | 单元测试（config.rs 旁） |
| tags 匹配、scenario-attrs 提取、capability 归属 | 单元测试（新模块旁） |
| morphology None/Some 两态 + serde 兼容 | partitioned.rs 单元 + sdd_bdd_compat_tests |

## 7. req_id 分配

落地时用 `llman sdd spec next-req-id` 现取两个全局未占用 id（分配器返回最低空闲 rN；写入前现查防并发漂移）。

## 8. 备选方案记录

- 「默认 tags 推导 + 可声明扩展」：对遵循 llman 自家约定的新项目零配置即真，但对 per-scenario 项目显示具误导性的 bound=0——被 xylitol 实测数据否决。
- 「两步走（先标签推导后配置化）」：中间态会让 xylitol 升级即退化，不值得。
