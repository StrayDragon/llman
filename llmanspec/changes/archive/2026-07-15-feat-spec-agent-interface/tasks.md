# Tasks: Spec Context Command

## 已完成

- [x] .context/ 目录结构（ContextMetadata/ContextIndex/Chunk）
- [x] compute_spec_hash() + check_freshness()
- [x] z_score_normalize() + cosine_sim()
- [x] is_pid_alive() + check_rebuild_lock()
- [x] `llman sdd context` 命令（单一确定性路径，无 fallback）
- [x] `llman sdd index rebuild` 命令（同步 + 异步）
- [x] `llman sdd index check` 命令（新鲜度检测）
- [x] scripts/embed_chunks.py（BGE-M3 embedding API helper）
- [x] 完整 rebuild 流程：scan specs → chunks → API → write index
- [x] context 语义查询：query embed + cosine sim + z-score tier
- [x] `list --specs --json` 扩展：purpose + validScope + health + staleness
- [x] `context --paths` 路径过滤

## 验证

- [x] `list --specs --json` → 34 specs with purpose + validScope
- [x] `context --task "config"` → config-paths #1 (z=4.10)
- [x] `context --task "error" --paths src/error.rs` → errors-exit #1 (z=4.11)
- [x] `context --task "coverage"` → tests-ci #1 (z=4.37)
- [x] `context` (no index) → quality: unavailable

## 依赖

- [x] `feat-spec-quality-triage` → 7 skill 模板更新已完成
