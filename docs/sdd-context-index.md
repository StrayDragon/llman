# `sdd context` / `sdd index` — semantic & agentic spec retrieval

`llman sdd context` finds which specs are relevant to a task (and/or file paths),
so an agent can read the right behavior contracts before making a change. It
supports two interchangeable backends, each with its own index.

## Backends

| Backend | How it retrieves | Needs a model? | Index location |
|---------|------------------|----------------|----------------|
| `pageindex` (**default**) | Agentic: an LLM navigates the spec tree via three tool calls (`list_specs` → `get_document_structure` → `get_spec_content`) and classifies specs into `direct`/`related` by reasoning. | Yes — a **chat** model that supports tool/function calling. | `llmanspec/.context/pageindex/tree.json` |
| `rag` (fallback) | Embedding RAG: each requirement is vectorized, the task is embedded, and specs are ranked by cosine similarity + z-score. | Yes — an **embedding** model. | `llmanspec/.context/rag/{chunks,specs,vectors.bin,metadata.toml}` |

Pick the backend with `--backend <rag|pageindex>` on `sdd context` and
`sdd index rebuild`, or preset it with `LLMAN_SDD_INDEX_BACKEND`.

```bash
# Build the default (pageindex) tree index — no model needed.
llman sdd index rebuild

# Build the rag embedding index.
llman sdd index rebuild --backend rag

# Query with the default (pageindex) backend.
llman sdd context --task "修改 validate 退出码" --paths "src/sdd/command.rs"

# Query with the rag backend.
llman sdd context --backend rag --task "..."
```

`llman sdd index check` reports the freshness of **each** backend's index.

## Output contract (shared by both backends)

```json
{
  "status": { "ok": true, "quality": "<semantic|agentic>", "qualityNote": null },
  "direct":  [{ "id": "<spec_id>", "reason": "..." }],
  "related": [{ "id": "<spec_id>", "reason": "..." }],
  "summary": { "totalSpecs": 35, "tierDirect": 1, "tierRelated": 2,
               "toolCalls": 11, "readRecommended": ["..."], "paths": ["..."] }
}
```

- `quality`: `semantic` (rag) / `agentic` (pageindex) / `unavailable` (index
  missing/stale/corrupted, with `status.errorKind`).
- pageindex's `reason` is LLM-generated; rag's is `"semantic match"` and also
  carries a `zScore`. pageindex's `summary` adds `toolCalls` (observability).
- When the requested backend's index is missing/stale, the command returns
  `quality=unavailable` with `errorKind` ∈
  `{index_missing, index_stale, index_corrupted}` and does **not** silently fall
  back to the other backend.

## Environment variables

| Variable | Backend | Purpose | Default |
|----------|---------|---------|---------|
| `LLMAN_SDD_INDEX_BACKEND` | both | Default backend when `--backend` is omitted | `pageindex` |
| `LLMAN_SDD_INDEX_OPENAI_API_HOST` | rag | Embedding API base URL | `http://coral:11534/v1` |
| `LLMAN_SDD_INDEX_OPENAI_API_KEY` | rag | Embedding API key | `omlx-gdpzzt2g5351xhqm` |
| `LLMAN_SDD_INDEX_MODEL` | rag | Embedding model | `bge-m3-mlx-8bit` |
| `LLMAN_SDD_INDEX_CHAT_API_HOST` | pageindex | Chat (tool-calling) API base URL. Falls back to `LLMAN_SDD_INDEX_OPENAI_API_HOST`. | — |
| `LLMAN_SDD_INDEX_CHAT_API_KEY` | pageindex | Chat API key. Falls back to `LLMAN_SDD_INDEX_OPENAI_API_KEY`. | — |
| `LLMAN_SDD_INDEX_CHAT_MODEL` | pageindex | Chat model (must support tool/function calling). **No default — required for `sdd context --backend pageindex`.** | — |
| `LLMAN_SDD_INDEX_DEBUG` | pageindex | Set to `1` to trace each agentic round (tool calls + final answer) to stderr for debugging model behavior. | unset |

## Notes

- Building the `pageindex` tree index is LLM-free — the spec IR is already a
  structured tree, so `sdd index rebuild` only needs to run when specs change.
- The agentic loop has a round limit; if a slow model hits it, the loop forces
  one final no-tools answer to salvage what it read, and marks the result
  `qualityNote` so the caller knows it was truncated.
- Legacy indexes that live directly under `llmanspec/.context/` (the pre-split
  layout) are still recognized as the `rag` index, so existing users don't lose
  their indexes. Re-running `sdd index rebuild --backend rag` migrates them to
  `.context/rag/`.
