<!-- llman-template-version: 1 -->
## Archive 冷备引导
- 当 archive 目录增长过大时，使用冷备维护：
  - 预览冻结候选：`llman sdd-legacy archive freeze --dry-run`
  - 冻结旧归档：`llman sdd-legacy archive freeze --before <YYYY-MM-DD> --keep-recent <N>`
  - 需要恢复时：`llman sdd-legacy archive thaw --change <YYYY-MM-DD-id>`
- freeze/thaw 仅用于日期归档目录（`YYYY-MM-DD-*`）；建议保留少量最近目录不冻结。
