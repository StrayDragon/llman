# Tasks: add-sdd-global-req-id-uniqueness

## 1. Validate + CLI

- [x] 1.1 主库全局 `req_id` 扫描：通用 validate 立即 ERROR 拦截 + 修复建议；保留单 doc 内重复检查
- [x] 1.2 `llman sdd spec next-req-id [--json]`（默认短 `rN`，不编 capability）
- [x] 1.3 `llman sdd spec add-req`：全局已占用（含自定义 tag）→ 非零退出
- [x] 1.4 `llman sdd spec resolve-req <id> [--json]`：归属 capability + title/statement
- [x] 1.5 单测：撞车、分配器、add-req 守卫、resolve-req

## 2. 本仓批量去重（短别名）

- [x] 2.1 检测跨 capability 冲突；保留一处，其余 remap 为新短 id（`project dedupe-req-ids`）
- [x] 2.2 同步 `.feature` `@req:` 与本 change delta / feature_delta（边界安全替换）
- [x] 2.3 `llman sdd validate --all --strict --no-interactive` 绿（specs 路径）

## 3. feature_delta harness

- [x] 3.1 BDD Given：跨 spec 重复 req_id；已占用全局 id
- [x] 3.2 绑定 authoring / collision feature 场景
- [x] 3.3 `llman sdd solidify add-sdd-global-req-id-uniqueness` 一致性门禁

## 4. 文档与兼容测试

- [x] 4.1 skills / AGENTS：短别名全局唯一 + next-req-id / resolve-req
- [x] 4.2 `tests/sdd_bdd_compat_tests.rs` smoke（next-req-id / resolve-req / dedupe-req-ids）
- [x] 4.3 fmt + clippy -D warnings + req_registry/smoke/bdd 新场景

## 5. 关闭变更

- [x] 5.1 change delta / feature_delta 就绪（主库撤回待 archive 合并，便于验收双管道）
- [x] 5.2 commit → archive（toon ops + feature_delta）
