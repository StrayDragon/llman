// Adapter that calls the `llman` binary for the rag / pageindex variants.
import type { RetrievalResult, Variant } from "./types.ts";

export interface LlmanOptions {
  /** Absolute path to the llman binary. */
  bin: string;
  /** Working directory (a project containing llmanspec/). */
  cwd: string;
  /** Extra env to merge onto process.env (API hosts/keys/config dir). */
  env?: Record<string, string>;
  /** Per-call timeout in ms. */
  timeoutMs?: number;
  /** top-K to pass (high so predictions aren't truncated by the default 10). */
  top?: number;
}

/** Run `llman sdd context` for one backend and parse its JSON. */
export async function runLlmanContext(
  backend: Extract<Variant, "rag" | "pageindex">,
  task: string,
  paths: string[] | undefined,
  opts: LlmanOptions,
): Promise<{ result: RetrievalResult; latencyMs: number }> {
  const args = [
    "sdd",
    "context",
    "--backend",
    backend,
    "--task",
    task,
    "--top",
    String(opts.top ?? 50),
  ];
  if (paths && paths.length > 0) {
    args.push("--paths", paths.join(","));
  }

  const t0 = performance.now();
  let exitCode = -1;
  let stdout = "";
  let stderr = "";
  try {
    const proc = Bun.spawn([opts.bin, ...args], {
      cwd: opts.cwd,
      env: { ...process.env, ...opts.env },
      stdout: "pipe",
      stderr: "pipe",
    });
    const timer = setTimeout(
      () => {
        try {
          proc.kill();
        } catch {
          /* already dead */
        }
      },
      opts.timeoutMs ?? 120_000,
    );
    [stdout, stderr] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    // `await proc.exit` can resolve to `undefined` under some Bun/inherit combos;
    // `proc.exited` is the reliable "wait until done" API. Fall back to 0 when
    // exit is undefined — a real failure surfaces via empty/missing JSON anyway.
    await proc.exited;
    exitCode = (await proc.exit) ?? 0;
    clearTimeout(timer);
  } catch (e) {
    const latencyMs = performance.now() - t0;
    return {
      result: {
        quality: "unavailable",
        error: `spawn failed: ${String(e)}`,
        direct: [],
        related: [],
      },
      latencyMs,
    };
  }
  const latencyMs = performance.now() - t0;

  const result = parseLlmanOutput(stdout, backend);
  // Only treat a bad exit as an error when we got NO usable result. A valid
  // JSON prediction wins over exit-code weirdness (some Bun/shell combos report
  // an undefined exit even on clean exits); surface the note only when truly
  // failed so it can't mislead reviewers.
  if (exitCode !== 0 && result.quality === "unavailable") {
    result.error = `llman exited ${exitCode}` + (stderr.trim() ? `: ${stderr.trim().slice(0, 300)}` : "");
  }
  return { result, latencyMs };
}

/** Pull the outermost JSON object out of stdout and map to RetrievalResult. */
export function parseLlmanOutput(
  stdout: string,
  backend: Extract<Variant, "rag" | "pageindex">,
): RetrievalResult {
  const jsonStr = extractJsonObject(stdout);
  if (!jsonStr) {
    return {
      quality: "unavailable",
      error: `no JSON object in llman stdout (got: ${stdout.slice(0, 200)})`,
      direct: [],
      related: [],
    };
  }
  let obj: any;
  try {
    obj = JSON.parse(jsonStr);
  } catch (e) {
    return {
      quality: "unavailable",
      error: `failed to parse llman JSON: ${String(e)}`,
      direct: [],
      related: [],
    };
  }
  const status = obj.status ?? {};
  const quality = (status.quality ?? "unavailable") as RetrievalResult["quality"];
  const note = status.qualityNote ?? null;
  return {
    quality,
    qualityNote: note,
    errorKind: status.errorKind,
    direct: (obj.direct ?? []).map((e: any) => ({
      id: String(e.id ?? ""),
      reason: e.reason,
      zScore: e.zScore,
    })),
    related: (obj.related ?? []).map((e: any) => ({
      id: String(e.id ?? ""),
      reason: e.reason,
      zScore: e.zScore,
    })),
    // pageindex embeds truncation info in qualityNote (round limit text).
    truncated: backend === "pageindex" && !!note,
    toolCalls: obj.summary?.toolCalls,
  };
}

function extractJsonObject(text: string): string | null {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) return null;
  return text.slice(start, end + 1);
}
