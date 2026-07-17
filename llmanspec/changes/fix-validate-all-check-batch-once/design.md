## Approach

在 `validate_spec_content_with_frontmatter_and_bdd` / `run_full_mode` 路径上，为 bulk validate 传入 `HashMap<expanded_command, CachedFullMode>`：

1. 首次遇到某 `expanded` 命令 → 真正 `spawn`，缓存 issues + 成败。
2. 后续相同 `expanded` → 不 spawn；写入短 INFO/ERROR（标明 reused），失败时仍计为 Error 以使该 spec `valid=false`。
3. 单 spec 路径不传 cache → 行为与今日一致。

按 expanded 字符串缓存即可同时覆盖「无占位符」与「占位符展开碰巧相同」；占位符展开不同则自然各自执行。

## Alternatives

- 仅检测模板是否含 `{feature_*}`，批量前跑一次：更简单，但无法覆盖「模板有占位符但多 spec 展开相同」的边角；选 expanded-key 缓存更通用。
- thread-local 全局缓存：污染同进程多次 validate；不如显式传入 bulk 作用域。

## Test strategy

- 单元测试：多 spec 目录 + `run_command` 写计数文件，经 bulk 路径断言 spawn 次数为 1。
- `@executable` feature：多 capability fixture + 计数 `run_command`，`validate --specs` 后断言计数文件行数为 1。
