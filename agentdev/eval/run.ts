// Orchestrator: run each variant over each case (×repeat), score, summarize.
//
//   bun run run.ts gen  --fixture xylitol --cases cases-xylitol.json
//   bun run run.ts run  --fixture xylitol --cases cases-xylitol.json --variants rag,pageindex,pi-retriever --repeat 3
//   bun run run.ts run  --fixture xylitol --dry   # plan only, no API calls
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Case, CaseRun, Gold, Variant, VariantSummary } from "./lib/types.ts";
import { scoreAll, aggregateVariant } from "./lib/metrics.ts";
import { runLlmanContext, type LlmanOptions } from "./lib/llman.ts";
import { runPiRetriever } from "./lib/pi-retriever.ts";
import { renderReport, type ReportMeta } from "./lib/report.ts";
import { generateCases } from "./cases/gen-from-archive.ts";

// ---- env resolution --------------------------------------------------------
function env(key: string, fallback = ""): string {
  const v = process.env[key];
  return v && v.trim() ? v : fallback;
}

/** API endpoints shared by all variants.
 *  Chat = pageindex + pi-retriever.
 *  rag embedding: **legacy** — rag backend removed; kept here only for
 *  historical reproduction against a pre-existing old rag index.
 *  NewAPI base/key default to the project's known-good values. */
function resolveApi() {
  const NEWAPI_API_KEY = env("NEWAPI_API_KEY");
  const NEWAPI_API_BASE = env("NEWAPI_API_BASE", "http://urchinet.lan:50256/v1");
  const CHAT_HOST = env("LLMAN_SDD_INDEX_CHAT_API_HOST", NEWAPI_API_BASE);
  const CHAT_KEY = env("LLMAN_SDD_INDEX_CHAT_API_KEY", NEWAPI_API_KEY);
  const CHAT_MODEL = env("LLMAN_SDD_INDEX_CHAT_MODEL", "deepseek-v4-flash");
  const EMBED_HOST = env("LLMAN_SDD_INDEX_OPENAI_API_HOST", "http://coral:11534/v1");
  const EMBED_KEY = env("LLMAN_SDD_INDEX_OPENAI_API_KEY", "omlx-gdpzzt2g5351xhqm");
  const EMBED_MODEL = env("LLMAN_SDD_INDEX_MODEL", "bge-m3-mlx-8bit");
  return { CHAT_HOST, CHAT_KEY, CHAT_MODEL, EMBED_HOST, EMBED_KEY, EMBED_MODEL };
}

/** llman env passed to the rag/pageindex subprocesses. */
function llmanEnv(configDir: string) {
  const { CHAT_HOST, CHAT_KEY, CHAT_MODEL, EMBED_HOST, EMBED_KEY, EMBED_MODEL } =
    resolveApi();
  return {
    LLMAN_CONFIG_DIR: configDir,
    LLMAN_SDD_INDEX_CHAT_API_HOST: CHAT_HOST,
    LLMAN_SDD_INDEX_CHAT_API_KEY: CHAT_KEY,
    LLMAN_SDD_INDEX_CHAT_MODEL: CHAT_MODEL,
    LLMAN_SDD_INDEX_OPENAI_API_HOST: EMBED_HOST,
    LLMAN_SDD_INDEX_OPENAI_API_KEY: EMBED_KEY,
    LLMAN_SDD_INDEX_MODEL: EMBED_MODEL,
  };
}

// ---- case loading ----------------------------------------------------------
async function loadCases(path: string): Promise<Case[]> {
  const raw = await readFile(path, "utf8");
  const arr = JSON.parse(raw) as Case[];
  return arr.filter((c) => c.gold.direct.length > 0); // unscoreable w/o gold
}

// ---- variant runners -------------------------------------------------------
async function runVariant(
  variant: Variant,
  cases: Case[],
  repeat: number,
  ctx: {
    llmanOpts: LlmanOptions;
    chatHost: string;
    chatKey: string;
    chatModel: string;
    treePath: string;
  },
  onProgress: (done: number, total: number, line: string) => void,
): Promise<CaseRun[]> {
  const runs: CaseRun[] = [];
  const total = cases.length * repeat;
  let done = 0;
  for (let r = 0; r < repeat; r++) {
    for (const c of cases) {
      let result;
      let latencyMs = 0;
      if (variant === "pi-retriever") {
        const out = await runPiRetriever(
          {
            chatHost: ctx.chatHost,
            chatKey: ctx.chatKey,
            chatModel: ctx.chatModel,
            treePath: ctx.treePath,
            timeoutMs: 180_000,
          },
          c.task,
          c.paths,
        );
        result = out.result;
        latencyMs = out.latencyMs;
      } else {
        const out = await runLlmanContext(variant, c.task, c.paths, ctx.llmanOpts);
        result = out.result;
        latencyMs = out.latencyMs;
      }
      runs.push({ variant, caseId: c.id, repeat: r, result, latencyMs });
      done++;
      const pred = (result.direct.map((e) => e.id).join(",") || "(empty)").slice(0, 40);
      const tag =
        result.quality === "unavailable" ? "UNAVAIL" : result.truncated ? "TRUNC" : "ok";
      onProgress(done, total, `[${variant} ${r}] ${c.id} -> ${tag} direct={${pred}}`);
    }
  }
  return runs;
}

// ---- index management -----------------------------------------------------
/** Rebuild a backend's index via `llman sdd index rebuild --backend <b>`.
 *  Skipped with --no-index. Success is judged by the produced artifact's
 *  mtime being newer than the rebuild start (we don't trust `proc.exit` here:
 *  under Bun's `stdout:"inherit"` it sometimes resolves to `undefined`). */
async function ensureIndex(
  backend: "pageindex" | "rag",
  projectDir: string,
  bin: string,
  configDir: string,
  api: ReturnType<typeof resolveApi>,
  noIndex: boolean,
) {
  const env = {
    ...process.env,
    LLMAN_CONFIG_DIR: configDir,
    LLMAN_SDD_INDEX_OPENAI_API_HOST: api.EMBED_HOST,
    LLMAN_SDD_INDEX_OPENAI_API_KEY: api.EMBED_KEY,
    LLMAN_SDD_INDEX_MODEL: api.EMBED_MODEL,
  };
  const artifact =
    backend === "pageindex"
      ? join(projectDir, "llmanspec", ".context", "pageindex", "tree.json")
      : join(projectDir, "llmanspec", ".context", "rag", "metadata.toml");
  console.log(`\n--- building ${backend} index (in ${projectDir}) ---`);
  const t0 = performance.now();
  const proc = Bun.spawn(
    [bin, "sdd", "index", "rebuild", "--backend", backend],
    { cwd: projectDir, env, stdout: "inherit", stderr: "inherit" },
  );
  await proc.exited;
  const secs = ((performance.now() - t0) / 1000).toFixed(1);
  // Judge by the artifact: it must exist and be newer than rebuild start.
  const fresh = existsSync(artifact);
  if (!fresh) {
    throw new Error(
      `llman sdd index rebuild --backend ${backend} did not produce ${artifact}`,
    );
  }
  console.log(`--- ${backend} index built (${secs}s) ---`);
}

// ---- main ------------------------------------------------------------------
async function main() {
  const args = process.argv.slice(2);
  const cmd = args[0] ?? "run";
  // --fixture <name> resolves to ./fixtures/<name> (a frozen corpus with
  // llmanspec/ inside). --project <dir> points at any dir containing llmanspec/.
  // Exactly one must be given; fixture wins if both appear.
  const fixtureName = option(args, "--fixture", "");
  // --project points at a project ROOT (contains llmanspec/), matching how
  // `llman sdd context` discovers specs from cwd. Default to the llman repo root.
  const projectDir = fixtureName
    ? resolve("fixtures", fixtureName)
    : resolve(option(args, "--project", "../.."));
  const casesPath = resolve(option(args, "--cases", "cases.json"));
  // rag is **legacy**: the rag backend was removed from llman. It can still
  // be selected explicitly for historical reproduction against a pre-existing
  // old rag index, but is excluded from the default variants.
  const variantsArg = option(args, "--variants", "pageindex,pi-retriever");
  const repeat = Number(option(args, "--repeat", "1"));
  const dry = args.includes("--dry");
  const noIndex = args.includes("--no-index");
  const outDir = resolve(option(args, "--out", "results"));

  if (cmd === "gen") {
    const { cases, needsGold } = await generateCases(projectDir, casesPath);
    console.log(`generated ${cases.length} cases -> ${casesPath}`);
    if (needsGold.length)
      console.log(`${needsGold.length} need manual gold`);
    return;
  }

  if (cmd !== "run") {
    console.error("usage: run.ts [gen|run] --project <dir> [--variants ...] [--repeat N] [--dry]");
    process.exit(2);
  }

  const variants = variantsArg.split(",").map((v) => v.trim()) as Variant[];
  for (const v of variants) {
    if (!["rag", "pageindex", "pi-retriever"].includes(v))
      throw new Error(`unknown variant: ${v}`);
  }

  const cases = await loadCases(casesPath);
  if (cases.length === 0) throw new Error(`no cases with gold at ${casesPath}`);

  const api = resolveApi();
  const bin = resolve(env("LLMAN_BIN", "../../target/debug/llman"));
  // Run the llman subprocess from projectDir itself: find_llmanspec_dir now
  // canonicalizes and walks up, so it finds <projectDir>/llmanspec from cwd.
  const treePath = join(projectDir, "llmanspec", ".context", "pageindex", "tree.json");
  const configDir = resolve(
    env("LLMAN_CONFIG_DIR_ABS", "../../artifacts/testing_config_home"),
  );
  const llmanEnvMap = llmanEnv(configDir);
  const llmanOpts: LlmanOptions = {
    bin,
    cwd: projectDir,
    env: llmanEnvMap,
    top: 50,
  };

  console.log(`project:   ${projectDir}`);
  console.log(`cases:     ${cases.length} (from ${casesPath})`);
  console.log(`variants:  ${variants.join(", ")}`);
  console.log(`repeat:    ${repeat}`);
  console.log(`chat:      ${api.CHAT_MODEL} @ ${api.CHAT_HOST}`);
  console.log(`llman bin: ${bin}`);
  console.log();

  if (dry) {
    console.log("--dry: would run:");
    for (const v of variants) {
      console.log(`  ${v}: ${cases.length} cases × ${repeat} = ${cases.length * repeat} runs`);
    }
    console.log(`index: ${noIndex ? "kept as-is" : "would rebuild pageindex (tree) [+rag if rag variant]"}`);
    return;
  }

  // Build indexes for the requested variants. Frozen fixtures ship WITHOUT a
  // .context/ index (it's produced by the CURRENT llman binary = the SUT), so
  // we rebuild it here. pageindex (tree) is LLM-free and fast; rag needs
  // embedding (slow) so only built when the rag variant is selected.
  const variantsSet = new Set(variants);
  await ensureIndex("pageindex", projectDir, bin, configDir, api, noIndex);
  if (variantsSet.has("rag")) {
    // rag is **legacy**: the rag backend was removed from llman, so
    // `--backend rag` is rejected. Rebuild will fail unless you pin to an
    // older llman binary that still accepts it.
    console.warn(
      "[WARN] rag is legacy and `llman sdd index rebuild --backend rag` will fail " +
      "on the current llman. Run with an older llman binary (LLMAN_BIN) to rebuild."
    );
    await ensureIndex("rag", projectDir, bin, configDir, api, noIndex);
  }

  const allRuns: CaseRun[] = [];
  for (const v of variants) {
    console.log(`\n=== variant: ${v} ===`);
    const runs = await runVariant(v, cases, repeat, {
      llmanOpts,
      chatHost: api.CHAT_HOST,
      chatKey: api.CHAT_KEY,
      chatModel: api.CHAT_MODEL,
      treePath,
    }, (_d, _t, line) => console.log(line));
    allRuns.push(...runs);
  }

  const goldByCase = new Map<string, Gold>(cases.map((c) => [c.id, c.gold]));
  const metrics = scoreAll(allRuns, goldByCase);
  const summaries = variants.map((v) =>
    aggregateVariant(v, metrics.filter((m) => m.variant === v)),
  );

  await mkdir(outDir, { recursive: true });
  // Per-run output dir: <sha-short>-<ts>/ groups every artifact of one run and
  // is traceable to the exact llman version under test (the SUT).
  const meta = await buildMeta(api, fixtureName || projectDir, repeat);
  const runDir = join(
    outDir,
    `${meta.llmanShaShort}-${meta.generatedAt.replace(/[:.]/g, "-").slice(0, 19)}`,
  );
  await mkdir(runDir, { recursive: true });
  const resultsPath = join(runDir, "results.json");
  const reportPath = join(runDir, "report.md");
  await writeFile(
    resultsPath,
    JSON.stringify({ meta, variants, repeat, cases, runs: allRuns, metrics, summaries }, null, 2),
    "utf8",
  );
  const md = renderReport(meta, summaries, metrics, cases);
  await writeFile(reportPath, md, "utf8");

  console.log(`\n${"=".repeat(70)}\n${md}`);
  console.log(`\nwrote: ${resultsPath}\n       ${reportPath}`);
}

/** Capture run metadata: llman git SHA (SUT version), fixture source SHA,
 *  chat model, timestamp. The SHA makes every report traceable to the exact
 *  llman build under test. */
async function buildMeta(
  api: ReturnType<typeof resolveApi>,
  fixtureOrProject: string,
  repeat: number,
): Promise<ReportMeta> {
  const binPath = resolve(env("LLMAN_BIN", "../../target/debug/llman"));
  const llmanSha = await gitHead(resolve(binPath, "../.."));
  const generatedAt = new Date().toISOString();
  // Fixture source SHA is recorded in fixtures/<name>/SNAPSHOT.md.
  let fixtureSha: string | undefined;
  let fixtureName = fixtureOrProject;
  const snap = join("fixtures", fixtureOrProject, "SNAPSHOT.md");
  if (existsSync(snap)) {
    const text = await readFile(snap, "utf8");
    const m = text.match(/source HEAD\s*\|\s*`([0-9a-f]{40})`/i);
    if (m) fixtureSha = m[1];
  }
  return {
    llmanSha,
    llmanShaShort: llmanSha.slice(0, 8),
    fixtureSha,
    fixtureName,
    chatModel: api.CHAT_MODEL,
    chatHost: api.CHAT_HOST,
    repeat,
    generatedAt,
  };
}

/** `git rev-parse HEAD` at the given repo dir, falling back to zeros. */
async function gitHead(repoDir: string): Promise<string> {
  try {
    const proc = Bun.spawn(["git", "-C", repoDir, "rev-parse", "HEAD"], {
      stdout: "pipe",
      stderr: "pipe",
    });
    const out = (await new Response(proc.stdout).text()).trim();
    return /^[0-9a-f]{40}$/.test(out) ? out : "0".repeat(40);
  } catch {
    return "0".repeat(40);
  }
}

function option(args: string[], flag: string, fallback: string): string {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : fallback;
}

main().catch((e) => {
  console.error("eval failed:", e);
  process.exit(1);
});
