// pi-retriever: a reference agentic retriever built on @earendil-works/pi-agent-core.
//
// It reuses the SAME pageindex tree.json and the SAME navigation system prompt
// as the Rust `pageindex` backend, so the only variable vs. that variant is the
// agent runtime (pi's retry/stop/parallel-tool logic vs. the hand-written 12-round
// loop + salvage turn). This isolates "loop implementation quality".
//
// Chat endpoint/key/model come from LLMAN_SDD_INDEX_CHAT_* — identical to the
// Rust pageindex backend, so the two are compared fairly.
import { Type } from "typebox";
import { Agent } from "@earendil-works/pi-agent-core";
import type { AgentTool } from "@earendil-works/pi-agent-core";
import { readFile } from "node:fs/promises";
import type { RetrievalResult } from "./types.ts";

/** Same navigation protocol as src/sdd/context/retrieve.rs (kept in sync). */
const SYSTEM_PROMPT = `You are a spec retrieval agent for an SDD (spec-driven development) project.
Given a task and optional file paths, find which specs are relevant.

NAVIGATION PROTOCOL (follow this order):
1. Call list_specs() to see all available spec documents and their purposes.
2. For specs whose purpose seems relevant to the task, call get_document_structure(spec_id) to see their requirement titles (cheap, no full text).
3. For requirements that look relevant, call get_spec_content(spec_id, req_ids) to read the full MUST/SHALL statements.
4. Finally, output ONLY a JSON object (no other text, no code fences):
   {"direct": [{"id": "<spec_id>", "reason": "<one sentence why this MUST be read>"}], "related": [{"id": "<spec_id>", "reason": "<one sentence>"}]}

CLASSIFICATION RULES:
- "direct" = specs whose behavior contract (any MUST/SHALL statement, command behavior, exit code, output format, validation rule, or CLI surface) is affected by this change.
- "related" = specs that provide useful context but whose contract won't change.
- If the task changes behavior, the governing spec MUST be in "direct" — do NOT leave "direct" empty just because you are unsure. An empty result means the task touches NO spec behavior at all (e.g. a typo or formatting fix in a non-spec file, a pure dependency version bump, or a file wholly outside every spec's scope).
- Decide "direct" vs "related" vs omit based on the requirement text you read, not on the spec's purpose line alone.
- Be precise: prefer fewer, well-reasoned entries over many guesses.`;

/** Safety cap mirroring the Rust MAX_TOOL_ROUNDS concept (but pi's turn != round;
 *  one pi turn can batch multiple tool calls). Generous so the model can finish. */
const MAX_TURNS = 24;

interface TreeReq {
  req_id: string;
  title: string;
  statement: string;
}
interface TreeDoc {
  spec_id: string;
  purpose: string;
  reqs: TreeReq[];
}
interface TreeIndex {
  docs: TreeDoc[];
  [k: string]: unknown;
}

function dispatch(name: string, args: any, tree: TreeIndex): string {
  switch (name) {
    case "list_specs": {
      const arr = tree.docs.map((d) => ({
        spec_id: d.spec_id,
        purpose: d.purpose,
        req_count: d.reqs.length,
      }));
      return JSON.stringify(arr);
    }
    case "get_document_structure": {
      const id = String(args?.spec_id ?? "");
      const d = tree.docs.find((x) => x.spec_id === id);
      if (!d) return JSON.stringify({ error: `spec_id ${JSON.stringify(id)} not found` });
      return JSON.stringify({
        spec_id: d.spec_id,
        purpose: d.purpose,
        reqs: d.reqs.map((r) => ({ req_id: r.req_id, title: r.title })),
      });
    }
    case "get_spec_content": {
      const id = String(args?.spec_id ?? "");
      const want = new Set((args?.req_ids ?? []).map(String));
      const d = tree.docs.find((x) => x.spec_id === id);
      if (!d) return JSON.stringify({ error: `spec_id ${JSON.stringify(id)} not found` });
      const result = d.reqs
        .filter((r) => want.size === 0 || want.has(r.req_id))
        .map((r) => ({ req_id: r.req_id, statement: r.statement }));
      return JSON.stringify(result);
    }
    default:
      return JSON.stringify({ error: `unknown tool: ${name}` });
  }
}

function buildTools(tree: TreeIndex): AgentTool<any>[] {
  return [
    {
      name: "list_specs",
      label: "List specs",
      description:
        "List all spec documents with metadata. Call this first to see what specs exist.",
      parameters: Type.Object({}),
      execute: async () => ({
        content: [{ type: "text", text: dispatch("list_specs", {}, tree) }],
        details: {},
      }),
    } as AgentTool<any>,
    {
      name: "get_document_structure",
      label: "Get document structure",
      description:
        "Get the tree structure of one spec (titles + req_ids only, no full text, to save tokens).",
      parameters: Type.Object({
        spec_id: Type.String({ description: "The spec id to inspect." }),
      }),
      execute: async (_id, params: any) => ({
        content: [
          { type: "text", text: dispatch("get_document_structure", params, tree) },
        ],
        details: {},
      }),
    } as AgentTool<any>,
    {
      name: "get_spec_content",
      label: "Get spec content",
      description:
        "Get the full statement text of specific requirements in a spec.",
      parameters: Type.Object({
        spec_id: Type.String({ description: "The spec id to read." }),
        req_ids: Type.Array(Type.String(), {
          description: "Requirement ids to fetch. Empty = all.",
        }),
      }),
      execute: async (_id, params: any) => ({
        content: [
          { type: "text", text: dispatch("get_spec_content", params, tree) },
        ],
        details: {},
      }),
    } as AgentTool<any>,
  ];
}

export interface PiRetrieverConfig {
  chatHost: string;
  chatKey: string;
  chatModel: string;
  treePath: string; // path to .context/pageindex/tree.json
  timeoutMs?: number;
}

/** Run the pi-retriever for one task. Returns a RetrievalResult in the same shape
 *  as the llman variants, plus tool-call count for cost comparison. */
export async function runPiRetriever(
  cfg: PiRetrieverConfig,
  task: string,
  paths: string[] | undefined,
): Promise<{ result: RetrievalResult; latencyMs: number }> {
  const raw = await readFile(cfg.treePath, "utf8");
  const tree: TreeIndex = JSON.parse(raw);

  // Hand-built OpenAI Chat-Completions-compatible model. We do NOT clone a
  // built-in model template (e.g. gpt-*): those use `openai-responses`, which
  // self-hosted OpenAI-compatible gateways (NEWAPI etc.) don't implement.
  // `openai-completions` targets /v1/chat/completions, which they do support.
  // The actual model id is cfg.chatModel (e.g. deepseek-v4-flash) — no gpt-*.
  const model = {
    id: cfg.chatModel,
    name: cfg.chatModel,
    api: "openai-completions",
    provider: "eval-chat",
    baseUrl: cfg.chatHost.replace(/\/+$/, ""),
    reasoning: false,
    input: ["text"],
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    contextWindow: 128_000,
    maxTokens: 4096,
  } as any;

  const tools = buildTools(tree);
  const toolCalls = { n: 0 };
  let turns = 0;
  const agent = new Agent({
    initialState: {
      systemPrompt: SYSTEM_PROMPT,
      model,
      thinkingLevel: "off" as any,
      tools,
    } as any,
    getApiKey: async (provider: string) =>
      provider === "eval-chat" ? cfg.chatKey : undefined,
    // Stop after MAX_TURNS turns to bound runaway loops (mirrors Rust's cap).
    // shouldStopAfterTurn is checked after each turn; returning true exits cleanly.
  } as any);

  agent.subscribe((e: any) => {
    if (e.type === "turn_end") {
      turns++;
    }
    if (e.type === "tool_execution_end") {
      toolCalls.n++;
    }
  });

  const userText =
    paths && paths.length > 0
      ? `Task: ${task}\nFile paths involved: ${paths.join(", ")}`
      : `Task: ${task}`;

  const t0 = performance.now();
  const ac = new AbortController();
  const timer = setTimeout(() => ac.abort(), cfg.timeoutMs ?? 180_000);
  let stopReason = "ok";
  try {
    // Drive via shouldStopAfterTurn by re-prompting is awkward; instead we rely on
    // the model naturally stopping (it emits final JSON with no tool calls).
    // The MAX_TURNS cap is enforced by aborting if exceeded (polled below).
    const stopWatcher = setInterval(() => {
      if (turns >= MAX_TURNS) {
        stopReason = "round_limit";
        ac.abort();
        clearInterval(stopWatcher);
      }
    }, 200);
    await agent.prompt(userText, undefined as any);
    clearInterval(stopWatcher);
  } catch (e) {
    stopReason = `error: ${String(e)}`;
  } finally {
    clearTimeout(timer);
  }
  const latencyMs = performance.now() - t0;

  // Extract assistant text from the WHOLE transcript (not just the last
  // message): with tool-calling models the final JSON may sit in an earlier
  // text block while the last turn is a trailing thinking/no-text message.
  const asstMessages = (agent.state.messages as any[]).filter(
    (m) => m.role === "assistant",
  );
  const allText = asstMessages
    .flatMap((m) => (m.content as any[]) ?? [])
    .filter((c) => c.type === "text")
    .map((c) => c.text ?? "");
  const lastText = [...allText].reverse().find((t) => t.includes("{")) ?? "";
  const text = lastText;
  if (process.env.PI_RETRIEVER_DEBUG) {
    process.stderr.write(
      `[pi-retriever] turns=${turns} toolCalls=${toolCalls.n} stop=${stopReason}\n`,
    );
    process.stderr.write(`[pi-retriever] text chunks=${allText.length} parseFrom=${JSON.stringify(text.slice(0, 200))}\n`);
  }

  const parsed = parseFinalAnswer(text);
  return {
    result: {
      quality: "agentic",
      direct: parsed.direct,
      related: parsed.related,
      toolCalls: toolCalls.n,
      truncated: stopReason === "round_limit",
      qualityNote: stopReason === "round_limit" ? "pi-retriever round limit" : null,
    },
    latencyMs,
  };
}

interface TierEntry {
  id: string;
  reason: string;
}

/** Parse the model's final {direct, related} JSON (mirrors retrieve.rs). */
export function parseFinalAnswer(content: string): {
  direct: TierEntry[];
  related: TierEntry[];
} {
  const jsonStr = extractJsonObject(content);
  if (!jsonStr) return { direct: [], related: [] };
  try {
    const ans = JSON.parse(jsonStr);
    const mk = (xs: any): TierEntry[] =>
      Array.isArray(xs)
        ? xs
            .map((e: any) => ({
              id: String(e?.id ?? ""),
              reason: String(e?.reason ?? ""),
            }))
            .filter((e: TierEntry) => e.id.length > 0)
        : [];
    return { direct: mk(ans.direct), related: mk(ans.related) };
  } catch {
    return { direct: [], related: [] };
  }
}

function extractJsonObject(text: string): string | null {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) return null;
  return text.slice(start, end + 1);
}
