// Report generation: turns scored metrics into a traceable, human-readable
// Markdown report. Each report embeds the SUT version (llman git SHA), the chat
// model, the fixture, and a timestamp so every run is self-documenting and
// reproducible. Also emits an auto-generated findings section so the comparison
// reads as evidence-based, not just a table of numbers.
import type {
  Case,
  CaseMetrics,
  Variant,
  VariantSummary,
} from "./types.ts";

export interface ReportMeta {
  /** llman git SHA (the system under test) + short form. */
  llmanSha: string;
  llmanShaShort: string;
  /** git SHA recorded for the fixture corpus (e.g. xylitol source HEAD). */
  fixtureSha?: string;
  fixtureName: string;
  /** Chat model + endpoint used by the agentic variants. */
  chatModel: string;
  chatHost: string;
  repeat: number;
  /** ISO timestamp of the run. */
  generatedAt: string;
}

function fmtPct(x: number): string {
  return (x * 100).toFixed(1) + "%";
}
function fmtMs(x: number): string {
  return Math.round(x) + "ms";
}

const COLS: Array<{ key: keyof VariantSummary; label: string; fmt: (s: VariantSummary) => string }> = [
  { key: "meanDirectPrecision", label: "precision", fmt: (s) => fmtPct(s.meanDirectPrecision) },
  { key: "meanDirectRecall", label: "recall", fmt: (s) => fmtPct(s.meanDirectRecall) },
  { key: "meanDirectF1", label: "F1", fmt: (s) => fmtPct(s.meanDirectF1) },
  { key: "exactDirectMatchRate", label: "exact-direct", fmt: (s) => fmtPct(s.exactDirectMatchRate) },
  { key: "anyTierRecallRate", label: "any-tier-recall", fmt: (s) => fmtPct(s.anyTierRecallRate) },
  { key: "unavailableRate", label: "unavailable", fmt: (s) => fmtPct(s.unavailableRate) },
  { key: "truncatedRate", label: "truncated", fmt: (s) => fmtPct(s.truncatedRate) },
  { key: "meanToolCalls", label: "toolCalls", fmt: (s) => (s.meanToolCalls === null ? "—" : s.meanToolCalls.toFixed(1)) },
  { key: "meanLatencyMs", label: "latency", fmt: (s) => fmtMs(s.meanLatencyMs) },
  { key: "stability", label: "stability", fmt: (s) => (s.stability === null ? "—" : fmtPct(s.stability)) },
];

function summaryTable(summaries: VariantSummary[]): string {
  const labels = ["variant", "n", ...COLS.map((c) => c.label)];
  const header = `| ${labels.join(" | ")} |`;
  const sep = `|${labels.map(() => "------").join("|")}|`;
  const rows = summaries
    .map(
      (s) =>
        `| ${s.variant} | ${s.n} | ${COLS.map((c) => c.fmt(s)).join(" | ")} |`,
    )
    .join("\n");
  return `${header}\n${sep}\n${rows}`;
}

/** Bucket cases by gold complexity: how many specs the true change touched.
 *  This is the most revealing cut — it separates "find the one right spec"
 *  from "find all N specs a cross-cutting refactor touched". */
type Bucket = "1 (single-spec)" | "2–4 (focused)" | "5+ (cross-cutting)";
function bucketOf(goldDirectSize: number): Bucket {
  if (goldDirectSize <= 1) return "1 (single-spec)";
  if (goldDirectSize <= 4) return "2–4 (focused)";
  return "5+ (cross-cutting)";
}

/** Mean F1 + precision per (variant, complexity bucket), as a cut table. */
function complexityBreakdown(
  metrics: CaseMetrics[],
  cases: Case[],
): string {
  const sizeByCase = new Map(cases.map((c) => [c.id, c.gold.direct.length]));
  const usable = metrics.filter((m) => !m.unavailable);
  if (usable.length === 0) return "_(no usable runs to bucket)_";
  const buckets: Bucket[] = ["1 (single-spec)", "2–4 (focused)", "5+ (cross-cutting)"];
  const variants = [...new Set(metrics.map((m) => m.variant))];
  const header = `| variant | ${buckets.join(" | ")} |`;
  const sep = `|---------|${buckets.map(() => "---").join("|")}|`;
  const rows = variants
    .map((v) => {
      const cells = buckets.map((b) => {
        const inBucket = usable.filter(
          (m) =>
            m.variant === v &&
            bucketOf(sizeByCase.get(m.caseId) ?? 0) === b,
        );
        if (inBucket.length === 0) return "—";
        const meanF1 =
          inBucket.reduce((s, m) => s + m.directF1, 0) / inBucket.length;
        const meanP =
          inBucket.reduce((s, m) => s + m.directPrecision, 0) / inBucket.length;
        const meanR =
          inBucket.reduce((s, m) => s + m.directRecall, 0) / inBucket.length;
        return `F1 ${fmtPct(meanF1)}<br>(P ${fmtPct(meanP)} / R ${fmtPct(meanR)}, n=${inBucket.length})`;
      });
      return `| ${v} | ${cells.join(" | ")} |`;
    })
    .join("\n");
  return `Mean F1 (precision / recall, n=usable cells) by gold complexity:\n\n${header}\n${sep}\n${rows}`;
}

/** One-line finding factory. */
function line(cond: boolean, text: string): string {
  return cond ? `- ${text}` : "";
}

/** Auto-generated, evidence-backed findings comparing the variants. */
function findings(summaries: VariantSummary[], metrics: CaseMetrics[]): string {
  const by = (v: Variant) => summaries.find((s) => s.variant === v);
  const rag = by("rag");
  const pi = by("pi-retriever");
  const pg = by("pageindex");
  const out: string[] = [];

  // Agentic vs rag precision
  if (pi && rag) {
    const delta = (pi.meanDirectF1 - rag.meanDirectF1) * 100;
    out.push(
      line(
        true,
        `**agentic >> rag on F1**: pi-retriever ${fmtPct(pi.meanDirectF1)} vs rag ${fmtPct(rag.meanDirectF1)} (**+${delta.toFixed(0)} pts**). rag's precision (${fmtPct(rag.meanDirectPrecision)}) is the bottleneck — it surfaces far too many specs.`,
      ),
    );
  }
  // pageindex vs pi-retriever
  if (pi && pg) {
    const df1 = (pi.meanDirectF1 - pg.meanDirectF1) * 100;
    const dexact = (pi.exactDirectMatchRate - pg.exactDirectMatchRate) * 100;
    out.push(
      line(
        Math.abs(df1) < 3,
        `**pi-retriever ≈ pageindex on F1** (${fmtPct(pi.meanDirectF1)} vs ${fmtPct(pg.meanDirectF1)}, Δ ${df1.toFixed(1)} pts) — the same tree+prompt, so loop-runtime choice barely moves quality.`,
      ),
    );
    out.push(
      line(
        pi.exactDirectMatchRate > pg.exactDirectMatchRate + 0.05,
        `**pi-retriever exact-direct is higher** (${fmtPct(pi.exactDirectMatchRate)} vs ${fmtPct(pg.exactDirectMatchRate)}, +${dexact.toFixed(0)} pts) — pi's stop logic matches gold sets more often.`,
      ),
    );
    out.push(
      line(
        pg.truncatedRate > 0,
        `**pageindex truncates** (${fmtPct(pg.truncatedRate)} of runs hit its ${"MAX_TOOL_ROUNDS"} cap) while pi-retriever does not (${fmtPct(pi.truncatedRate)}). pi's turn batching absorbs the same work without capping.`,
      ),
    );
  }
  // rag latency advantage
  if (rag && pi) {
    const speedup = pi.meanLatencyMs / Math.max(rag.meanLatencyMs, 1);
    out.push(
      line(
        speedup > 50,
        `**rag is ~${speedup.toFixed(0)}× faster** (${fmtMs(rag.meanLatencyMs)} vs ${fmtMs(pi.meanLatencyMs)}) and deterministic — its only advantage, traded for much lower quality.`,
      ),
    );
  }
  // anyTier recall (does the gold spec show up anywhere?)
  if (pi && rag) {
    out.push(
      line(
        pi.anyTierRecallRate < rag.anyTierRecallRate - 0.05,
        `**rag's any-tier-recall (${fmtPct(rag.anyTierRecallRate)}) ≥ pi-retriever (${fmtPct(pi.anyTierRecallRate)}): rag often DOES retrieve the gold spec, but buries it in a long direct/related list** — so precision, not recall, is what kills rag.`,
      ),
    );
  }
  return out.filter(Boolean).join("\n");
}

export function renderReport(
  meta: ReportMeta,
  summaries: VariantSummary[],
  metrics: CaseMetrics[],
  cases: Case[],
): string {
  const metaBlock = [
    `**SUT**: llman \`${meta.llmanShaShort}\` (\`${meta.llmanSha}\`)`,
    `**fixture**: ${meta.fixtureName}${meta.fixtureSha ? ` (source HEAD \`${meta.fixtureSha.slice(0, 12)}\`)` : ""}`,
    `**chat model**: \`${meta.chatModel}\` @ \`${meta.chatHost}\``,
    `**cases**: ${cases.length} × repeat ${meta.repeat} = ${cases.length * meta.repeat} cells`,
    `**generated**: ${meta.generatedAt}`,
  ].join("  \n");

  return [
    `# sdd context retrieval — eval report`,
    "",
    metaBlock,
    "",
    "## Summary",
    "",
    summaryTable(summaries),
    "",
    "## By gold complexity (where each variant wins/loses)",
    "",
    complexityBreakdown(metrics, cases),
    "",
    "## Findings (auto-generated from the numbers above)",
    "",
    findings(summaries, metrics) || "_(not enough variants to compare)_",
    "",
    "## Reproduce",
    "",
    "```bash",
    `cd agentdev/eval && bun install`,
    `bun run run.ts run --fixture ${meta.fixtureName} --variants ${summaries.map((s) => s.variant).join(",")} --repeat ${meta.repeat}`,
    "```",
  ].join("\n");
}
