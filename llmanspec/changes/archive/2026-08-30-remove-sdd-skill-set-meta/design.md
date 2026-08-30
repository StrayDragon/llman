# Design: remove-sdd-skill-set-meta

## 决策

- **D1 无消费即退役**：skill_set 在安装产物上零读取方（r90 读 config、
  r96 读 extra_skill_* 变量、清理读前缀）——装饰性元数据按零兼容原则删除。
- **D2 r95 整条删除而非再修订**：其门禁主体（llman_sdd/skill_set）消失后
  规则无剩余语义；unrendered-jinja 检查本就不在 r95 条文内，保留为实现层。
- **D3 托管边界不变**：清理与识别继续依赖 `llman-sdd-` 前缀（r90），
  frontmatter 是否含 llman_sdd 不参与任何判定。
- **D4 version 保留**：`metadata.version` 有消费方（check-skills-version.py），
  不在本次范围。

## 测试边界

- 单测：skill_consistency 重写为 jinja-only（残留检查正向/负向）；
  bdd_steps/it fixture 去除 metadata 写入；模板 parity 门禁照旧。
