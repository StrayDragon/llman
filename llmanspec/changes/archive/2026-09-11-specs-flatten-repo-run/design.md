# Design

沿用已归档 change `2026-09-11-specs-flat-layout` 的设计与决策（D1-D12），本 change 不引入新设计：纯机械执行 `migrate --kind specs-flatten`，仅路径迁移 + `# scope:` 自引用改写，合约内容零变更。
