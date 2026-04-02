## 1. 配置与命令面

- [x] 1.1 在 `src/sdd/project/config.rs`、schema 生成代码与 `artifacts/schema/configs/en/llmanspec-config.schema.json` 中新增 `spec_style` 枚举字段，并让 `llman sdd init` 显式写入 `spec_style: ison`
- [x] 1.2 为所有读取或改写 spec/delta 的 `llman sdd` 命令引入“必须已声明 `spec_style`”的加载路径，缺失/非法配置时直接失败并输出明确提示
- [x] 1.3 在 `src/sdd/command.rs` 中新增 `llman sdd convert` 子命令及参数解析，并把 `--pretty-ison` 的适用范围限制到 `ison` 项目

## 2. 共享语义模型与风格后端

- [x] 2.1 抽取主 spec / delta spec 的共享语义 IR 与 style backend trait，重构 `src/sdd/spec/parser.rs` / `validation.rs` 使业务逻辑不再硬编码 ISON
- [x] 2.2 保留并整理现有 ISON backend：继续复用 `ison-rs` 并把已知解析缺陷 workaround 收敛在适配层；保持 canonical block merge 与稳定序列化
- [x] 2.3 实现 TOON backend（固定使用 `serde_toon_format`）：支持主 spec 与 delta spec 的 fenced ` ```toon ` 解析、严格校验与稳定写回，并补齐依赖与基准 fixtures
- [x] 2.4 实现 YAML backend 的“语义解析层”：支持 fenced ` ```yaml ` 解析到 IR、错误定位、以及用于转换/新建文件的确定性序列化输出
- [x] 2.5 实现 YAML backend 的“lossless 写回层”：基于 SDD-aware overlay planner（按 `req_id`、`(req_id,id)` 等语义标识匹配）生成递归更新计划，并用 `yamlpatch` 以保留注释/格式的方式应用到原始 YAML 源文本
- [x] 2.6 为 YAML lossless 写回定义回退策略：当 `yamlpatch` 应用失败时，退化为“仅重写 fenced YAML payload”的确定性重写（保留外围 Markdown），并在输出中明确提示注释可能丢失

## 3. 命令集成与转换流程

- [x] 3.1 让 `show` / `list` / `validate` 按项目 `spec_style` 选择解析后端，并在风格不匹配时报告“期望风格 vs 实际内容”
- [x] 3.2 重构 `src/sdd/change/archive.rs` 与 authoring helpers，使 archive merge、`llman sdd spec ...`、`llman sdd delta ...` 都基于共享 IR 并按配置风格写回；对 `yaml` 项目优先使用 lossless overlay 写回而不是整文档重写
- [x] 3.3 实现 `llman sdd convert --to <style> --project`：预检查全部源文件、批量转换主 spec 与 active change delta spec、成功后再更新 `llmanspec/config.yaml`
- [x] 3.4 实现 `llman sdd convert --to <style> --file <path> [--output <path>]` 与 `--dry-run`，支持单文件审阅/迁移且不隐式改写项目配置

## 4. 模板、技能与帮助文本

- [x] 4.1 更新 `templates/sdd/**`、`llmanspec/AGENTS.md` 渲染与 `llman sdd update-skills` 产物，使 spec/delta 示例和指导跟随项目 `spec_style`
- [x] 4.2 更新帮助文本、错误提示与相关文档，明确 `toon` / `yaml` 为 experimental，并说明普通读写路径不会隐式转换风格

## 5. 测试与验证

- [x] 5.1 为配置与命令面补充测试：覆盖 `spec_style` 缺失/非法、`init` 默认写入、`convert` clap 解析、`--pretty-ison` 非 ISON 项目报错
- [x] 5.2 为三种风格补充解析/校验/JSON 输出/authoring/archive 集成测试，确认相同语义在 `ison` / `toon` / `yaml` 下结果一致
- [x] 5.3 为转换流程补充测试：覆盖单文件 stdout/输出文件、项目范围成功切换、目标重解析失败时不更新 `llmanspec/config.yaml`
- [x] 5.4 为 YAML lossless 写回补充 preservation fixtures：覆盖注释、空白、缩进、键顺序的保留，以及 overlay 更新不会重排无关段落
- [x] 5.5 运行 `openspec validate support-multi-style-sdd-specs --strict --no-interactive` 与相关 Rust 检查，修复验证和测试失败项
