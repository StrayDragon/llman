# Tasks: harness bound 可声明口径

垂直切片：每个 task 独立可验证，一刀切穿 schema→解析→计算→测试。

## T1 配置词汇表 `bdd.bindings`

- [x] `src/sdd/project/config.rs`：`BddConfig` 新增 `bindings: Option<Vec<BindingSource>>`；
      `BindingSource` 枚举 `{ kind: tags, tags: Vec<String> }` | `{ kind: scenario-attrs, files: Vec<String> }`，
      serde tag = `kind`，camelCase 兼容；未知 kind / 空 tags / 空 files → 解析错误（fail loudly）。
- [x] 单元测试：合法两态反序列化、未知 kind 拒绝、缺省（无 bindings 段）= None。
- [x] 再生成 `artifacts/schema/configs/en/llmanspec-config.schema.json` 并跑 schema 校验
      （遵循 config-schemas r49/r73 既有合约）。

## T2 绑定解析器

- [x] 新模块（如 `src/sdd/spec/bindings.rs`）：输入 `&[BindingSource]` + spec 目录 +
      仓库根，输出每 capability 的 bound 集 `HashSet<(feature_rel_path, scenario_name)>`。
- [x] `tags` 源：场景 tags 含全部声明标签即命中（`@` 前缀归一化）。
- [x] `scenario-attrs` 源：glob 展开文件 → 提取 `#[scenario(...)]` 块内 `path = "…"` /
      `name = "…"` 字面量对（容忍多行）；feature 路径按 `<cap>/` 段归属 capability；
      specs 目录外路径忽略；`index=` 选择器绑定忽略并记警告。
- [x] 单元测试：两源并集去重、多行属性、目录外路径过滤、glob 无匹配的空结果语义。

## T3 morphology 拆分与 list/show 输出 [blocked-by: T1, T2]

- [x] `src/sdd/spec/partitioned.rs`：`Morphology` 加 `harness_bound_count: Option<usize>`、
      `harness_unbound_count: Option<usize>`（camelCase 序列化；B+U=total 不变式）。
- [x] `src/sdd/shared/list.rs`：声明源时文本行输出 `harness-bound {b} harness-unbound {u}`；
      未声明时维持现文本。JSON 两键恒在、未声明为 null。
- [x] `src/sdd/shared/show.rs`：Morphology 行同口径同步。
- [x] 集成测试（TempDir + TestProcess）：未声明 → 输出与旧形态一致且 JSON 新键为 null；
      声明 tags 源 → 文本/JSON 拆分正确；声明 scenario-attrs 源 → 同。
- [x] `tests/sdd_bdd_compat_tests.rs`：旧 JSON（无新键）可反序列化（serde 向后兼容断言）。

## T4 feature 场景与文档同步 [blocked-by: T3]

- [x] `llmanspec/specs/sdd-workflow/list-context-triage.feature`：新增 fast mode 场景
      （@req 挂新 requirement）：未声明不拆列 / 声明后拆列（CLI 边界文档化，Rust 测试覆盖）。
- [x] 根 AGENTS.md：更正过时的「`#[scenario(path=…, name=…)]` 绑定」描述为
      `scenarios!` 目录发现 + tag 表达式现实，并提及 `bdd.bindings` 可声明口径。

## T5 全量门禁

- [x] `just check`（fmt/clippy/test）；`just check-sdd-templates` 若模板未动则跳过说明。
