// Pure metric functions. No I/O, no network — fully unit-testable.
import type {
  CaseMetrics,
  CaseRun,
  Gold,
  RetrievalResult,
  Variant,
  VariantSummary,
} from "./types.ts";

export function ids(result: RetrievalResult, tier: "direct" | "related"): string[] {
  return (tier === "direct" ? result.direct : result.related)
    .map((e) => e.id)
    .filter((id) => typeof id === "string" && id.length > 0);
}

export function predictedDirect(result: RetrievalResult): string[] {
  return dedupe(ids(result, "direct"));
}

function dedupe(xs: string[]): string[] {
  return [...new Set(xs)];
}

/** Precision = |pred ∩ gold| / |pred|. Conventionally 0 when pred is empty,
 *  even if gold is also empty (we never reward an empty prediction). */
export function precision(pred: string[], gold: string[]): number {
  if (pred.length === 0) return 0;
  const g = new Set(gold);
  const tp = pred.filter((x) => g.has(x)).length;
  return tp / pred.length;
}

/** Recall = |pred ∩ gold| / |gold|. Both empty => 1 (trivially satisfied). */
export function recall(pred: string[], gold: string[]): number {
  if (gold.length === 0) return pred.length === 0 ? 1 : 0;
  const g = new Set(gold);
  const tp = pred.filter((x) => g.has(x)).length;
  return tp / gold.length;
}

export function f1(p: number, r: number): number {
  if (p + r === 0) return 0;
  return (2 * p * r) / (p + r);
}

/** Score a single run against its gold. */
export function scoreCase(
  variant: Variant,
  caseId: string,
  repeat: number,
  result: RetrievalResult,
  gold: Gold,
  latencyMs: number,
): CaseMetrics {
  const pred = predictedDirect(result);
  const predAnyTier = dedupe([...ids(result, "direct"), ...ids(result, "related")]);
  const p = precision(pred, gold.direct);
  const r = recall(pred, gold.direct);
  const exact = setEq(pred, gold.direct);
  const anyTier = gold.direct.every((g) => predAnyTier.includes(g)) ? 1 : 0;
  return {
    variant,
    caseId,
    repeat,
    unavailable: result.quality === "unavailable",
    directPrecision: p,
    directRecall: r,
    directF1: f1(p, r),
    exactDirectMatch: exact ? 1 : 0,
    anyTierRecall: anyTier,
    toolCalls: result.toolCalls,
    truncated: !!result.truncated,
    latencyMs,
    error: result.error,
  };
}

function setEq(a: string[], b: string[]): boolean {
  const sa = new Set(a);
  const sb = new Set(b);
  return sa.size === sb.size && [...sa].every((x) => sb.has(x));
}

function mean(xs: number[]): number {
  if (xs.length === 0) return 0;
  return xs.reduce((s, x) => s + x, 0) / xs.length;
}

/** Aggregate all runs of one variant into a summary. */
export function aggregateVariant(
  variant: Variant,
  metrics: CaseMetrics[],
): VariantSummary {
  // `unavailable` is the real gate (index missing / config error / no JSON).
  // `error` is an informational diagnostic (e.g. odd exit code) and must NOT
  // exclude a run that did produce a usable prediction.
  const usable = metrics.filter((m) => !m.unavailable);
  const cells = metrics.length;
  return {
    variant,
    n: new Set(metrics.map((m) => m.caseId)).size,
    cells,
    unavailableRate: metrics.filter((m) => m.unavailable).length / cells,
    truncatedRate: metrics.filter((m) => m.truncated).length / cells,
    meanDirectPrecision: mean(usable.map((m) => m.directPrecision)),
    meanDirectRecall: mean(usable.map((m) => m.directRecall)),
    meanDirectF1: mean(usable.map((m) => m.directF1)),
    exactDirectMatchRate: mean(usable.map((m) => m.exactDirectMatch)),
    anyTierRecallRate: mean(usable.map((m) => m.anyTierRecall)),
    meanToolCalls:
      usable.some((m) => typeof m.toolCalls === "number")
        ? mean(usable.map((m) => m.toolCalls ?? 0))
        : null,
    meanLatencyMs: mean(metrics.map((m) => m.latencyMs)),
    stability: stabilityOf(variant, metrics),
  };
}

/** Fraction of cases whose predicted direct set was identical across all repeats.
 *  null when there's only one repeat. */
function stabilityOf(variant: Variant, metrics: CaseMetrics[]): number | null {
  const deterministic = variant === "rag";
  if (deterministic) return 1;
  const maxRepeat = Math.max(0, ...metrics.map((m) => m.repeat));
  if (maxRepeat < 1) return null;
  const byCase = new Map<string, CaseMetrics[]>();
  for (const m of metrics) {
    if (m.unavailable || m.error) continue;
    const arr = byCase.get(m.caseId) ?? [];
    arr.push(m);
    byCase.set(m.caseId, arr);
  }
  const rates: number[] = [];
  for (const arr of byCase.values()) {
    // stability per case = best direct-set agreement fraction across repeats
    // approximated by F1 variance: 1 if all F1 identical, else lower.
    const f1s = arr.map((m) => m.directF1);
    const allSame = f1s.every((f) => Math.abs(f - f1s[0]) < 1e-9);
    rates.push(allSame ? 1 : 0);
  }
  return rates.length ? mean(rates) : null;
}

/** Turns a list of (run, gold) pairs into metrics, grouped per variant. */
export function scoreAll(
  runs: CaseRun[],
  goldByCase: Map<string, Gold>,
): CaseMetrics[] {
  const out: CaseMetrics[] = [];
  for (const run of runs) {
    const gold = goldByCase.get(run.caseId);
    if (!gold) {
      continue; // unknown case, skip
    }
    out.push(
      scoreCase(
        run.variant,
        run.caseId,
        run.repeat,
        run.result,
        gold,
        run.latencyMs,
      ),
    );
  }
  return out;
}

// ---- self-test -------------------------------------------------------------
function assert(cond: boolean, msg: string) {
  if (!cond) throw new Error("assert failed: " + msg);
}

export function selfTest() {
  assert(precision(["a", "b"], ["a"]) === 0.5, "precision basic");
  assert(precision([], ["a"]) === 0, "precision empty pred");
  assert(recall(["a"], ["a", "b"]) === 0.5, "recall basic");
  assert(recall([], []) === 1, "recall both empty");
  assert(recall(["a"], []) === 0, "recall gold empty pred nonempty");
  assert(f1(1, 0) === 0, "f1 zero");
  assert(Math.abs(f1(1, 1) - 1) < 1e-9, "f1 perfect");

  // a typical case: pred direct={sdd-workflow}, gold direct={sdd-workflow}
  const m = scoreCase(
    "pageindex",
    "c1",
    0,
    {
      quality: "agentic",
      direct: [{ id: "sdd-workflow" }],
      related: [{ id: "errors-exit" }],
      toolCalls: 9,
    },
    { direct: ["sdd-workflow"], related: ["errors-exit"] },
    1234,
  );
  assert(m.directPrecision === 1, "case precision");
  assert(m.directRecall === 1, "case recall");
  assert(m.exactDirectMatch === 1, "case exact");
  assert(m.anyTierRecall === 1, "case anyTier");
  assert(m.toolCalls === 9, "case toolCalls");

  // tier slip: gold direct={a}, pred direct={b} related={a} -> recall 0 but anyTier 1
  const slip = scoreCase(
    "pageindex",
    "c2",
    0,
    { quality: "agentic", direct: [{ id: "b" }], related: [{ id: "a" }] },
    { direct: ["a"], related: [] },
    10,
  );
  assert(slip.directRecall === 0, "slip recall 0");
  assert(slip.anyTierRecall === 1, "slip anyTier 1");
  assert(slip.exactDirectMatch === 0, "slip not exact");

  // stability: 2 repeats, same f1 -> 1; different -> 0
  const stab = aggregateVariant("pageindex", [
    scoreCase("pageindex", "c", 0, { quality: "agentic", direct: [{ id: "a" }], related: [] }, { direct: ["a"], related: [] }, 1),
    scoreCase("pageindex", "c", 1, { quality: "agentic", direct: [{ id: "a" }], related: [] }, { direct: ["a"], related: [] }, 1),
  ]);
  assert(stab.stability === 1, "stab same");

  console.log("metrics self-test OK");
}

if (import.meta.path === Bun.main) {
  selfTest();
}
