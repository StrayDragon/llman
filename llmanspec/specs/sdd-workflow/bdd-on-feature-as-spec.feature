# language: zh-CN
# 对应 spec: sdd-workflow r51-r56 — Feature-as-Spec（BDD-on）模式：.feature 文件承载行为规格，
# glob 自动发现无需注册表；spec.toon 仅留 kind/name/purpose；fast/full 双层校验；
# locale 驱动 Gherkin 语言；BDD-on 门控与旧路径并存；BDD-on 退休 valid_scope；
# BDD-on 归档复制 .feature 文件。
功能: BDD-on（feature-as-spec）模式
  场景: 目录即 spec，validate 自动发现 feature
    假如 config.yaml 含 bdd: 段且 specs/cli/ 下有 status.feature
    当 用户运行 llman sdd validate cli
    那么validate 自动发现 status.feature 并解析
    而且不要求任何注册表或 spec.toon 引用表

  场景: 新增 feature 无需注册
    假如 config.yaml 含 bdd: 段
    当 用户将 new-behavior.feature 放入 specs/cli/ 目录
    那么 validate 自动发现该文件
    而且无需修改 spec.toon 或任何 manifest

  场景: spec.toon 仅校验元数据
    假如 config.yaml 含 bdd: 段
    当 validate 解析 specs/cli/spec.toon
    那么仅校验 kind/name/purpose
    而且不要求 valid_scope

  场景: 迁移中状态合法
    假如 config.yaml 含 bdd: 段且 specs/cli/ 有 .feature 且 requirements 剩余
    当 用户运行 llman sdd validate cli
    那么校验通过（迁移中状态合法）

  场景: 空 spec 报错
    假如 config.yaml 含 bdd: 段且 specs/cli/ 无 .feature 且 requirements 为空
    当 用户运行 llman sdd validate cli
    那么校验失败并报 ERROR（空 spec）

  场景: 配了 BDD 时 validate 默认跑 runner
    假如 config.yaml 含 bdd: 段且 run_command 为 cargo test --features bdd
    当 用户运行 llman sdd validate errors-exit
    那么 Gherkin 解析通过后自动执行 bdd.run_command
    而且 exit 0 时输出 Full mode: N feature(s) parsed, BDD check passed.

  场景: --no-check 跳过 runner
    假如 config.yaml 含 bdd: 段
    当 用户运行 llman sdd validate errors-exit --no-check
    那么仅做 Gherkin 解析
    而且不执行 bdd.run_command
    而且输出不含 Full mode

  场景: --check 在 BDD-on 时为默认行为
    假如 config.yaml 含 bdd: 段
    当 用户运行 llman sdd validate errors-exit --check
    那么行为与不传任何 flag 完全一致（默认已自动执行 runner）

  场景: BDD-off 时 --check 给出 INFO 提示
    假如 config.yaml 无 bdd: 段
    当 用户运行 llman sdd validate cli --check
    那么校验通过（Gherkin/prose 路径不变）
    而且输出 INFO：--check 无 BDD 配置时无效果

  场景: locale 驱动中文 Gherkin
    假如 config.yaml locale 为 zh-Hans 且 feature 使用中文关键字
    当 用户运行 llman sdd validate cli
    那么 gherkin 解析使用 lang=zh-CN
    而且校验通过

  场景: BDD-off 路径不变
    假如 config.yaml 无 bdd: 段
    当 用户运行 llman sdd validate cli
    那么走现有 prose 校验路径
    而且行为与新增本功能前完全一致

  场景: BDD-on staleness 不依赖 valid_scope
    假如 config.yaml 含 bdd: 段且 specs/cli/ 下某 .feature 被改动
    当 validate 做 staleness 校验
    那么判定该 cli spec 被触及
    而且不依赖 valid_scope 匹配

  场景: BDD-on 归档复制 feature 文件
    假如 config.yaml 含 bdd: 段且 change/specs/cli/ 含 status.feature
    当 用户运行 llman sdd archive run <change>
    那么status.feature 被复制到 specs/cli/
    而且随后 change 目录被 rename 进 archive

  场景: BDD-on 归档 feature 冲突中止
    假如 config.yaml 含 bdd: 段且目标 specs/cli/status.feature 已存在
    当 用户运行 llman sdd archive run <change>
    那么命令报错中止
    而且不覆盖目标文件
