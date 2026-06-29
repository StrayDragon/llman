# `sdd context` / `sdd index` — agentic spec retrieval

`llman sdd context` finds which specs are relevant to a task (and/or file paths),
so an agent can read the right behavior contracts before making a change. The sole
backend is **pageindex**.

## Backend

| Backend | How it retrieves | Needs a model? | Index location |
|---------|------------------|----------------|----------------|
| `pageindex` | Agentic: an LLM navigates the spec tree via three tool calls (`list_specs` → `get_document_structure` → `get_spec_content`) and classifies specs into `direct`/`related` by reasoning. | Yes — a **chat** model that supports tool/function calling. | `llmanspec/.context/pageindex/tree.json` |

`--backend` is accepted for compatibility (`pageindex` is the only valid value)
and can be preset with `LLMAN_SDD_INDEX_BACKEND`.

```bash
# Build the pageindex tree index — no model needed.
llman sdd index rebuild

# Query with the default pageindex backend.
llman sdd context --task "修改 validate 退出码" --paths "src/sdd/command.rs"
```

`llman sdd index check` reports index freshness.

## Output contract

```json
{
  "status": { "ok": true, "quality": "agentic", "qualityNote": null },
  "direct":  [{ "id": "<spec_id>", "reason": "..." }],
  "related": [{ "id": "<spec_id>", "reason": "..." }],
  "summary": { "totalSpecs": 35, "tierDirect": 1, "tierRelated": 2,
               "toolCalls": 11, "readRecommended": ["..."], "paths": ["..."] }
}
```

- `quality`: `agentic` (pageindex) / `unavailable` (index missing/stale/corrupted,
  with `status.errorKind`).
- `reason` is LLM-generated.
- When the index is missing/stale, the command returns `quality=unavailable` with
  `errorKind` ∈ `{index_missing, index_stale, index_corrupted}` and a concise
  `qualityNote` (e.g. `index missing; run \`llman sdd index rebuild\``).
  It does **not** silently fall back.

## Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `LLMAN_SDD_INDEX_BACKEND` | Explicitly declare the backend (`pageindex`). Other values are rejected. | unset |
| `LLMAN_SDD_INDEX_CHAT_API_HOST` | Chat (tool-calling) API base URL. Falls back to `LLMAN_SDD_INDEX_OPENAI_API_HOST`. | — |
| `LLMAN_SDD_INDEX_CHAT_API_KEY` | Chat API key. Falls back to `LLMAN_SDD_INDEX_OPENAI_API_KEY`. | — |
| `LLMAN_SDD_INDEX_CHAT_MODEL` | Chat model (must support tool/function calling). **No default — required for `sdd context`.** | — |
| `LLMAN_SDD_INDEX_DEBUG` | Set to `1` to trace each agentic round (tool calls + final answer) to stderr for debugging model behavior. | unset |

## Notes

- Building the pageindex tree index is LLM-free — the spec IR is already a
  structured tree, so `sdd index rebuild` only needs to run when specs change.
- The agentic loop has a round limit; if a slow model hits it, the loop forces
  one final no-tools answer to salvage what it read, and marks the result
  `qualityNote` so the caller knows it was truncated.
