// Shared types for the sdd-context retrieval eval harness.
// All variants (rag / pageindex / pi-retriever) normalize to RetrievalResult.

/** A retrieval prediction from any variant, normalized to one shape. */
export interface RetrievalResult {
  /** "semantic" (rag) | "agentic" (pageindex/pi-retriever) | "unavailable" */
  quality: "semantic" | "agentic" | "unavailable";
  qualityNote?: string | null;
  /** Present when quality === "unavailable" (index_missing/index_stale/...). */
  errorKind?: string;
  /** Error message if the variant crashed (network/parse), kept out of quality. */
  error?: string;
  direct: TierEntry[];
  related: TierEntry[];
  /** pageindex/pi-retriever only: number of tool calls made. */
  toolCalls?: number;
  /** pageindex/pi-retriever only: hit the round limit? */
  truncated?: boolean;
}

export interface TierEntry {
  id: string;
  reason?: string;
  /** rag only: z-score. */
  zScore?: number;
}

/** A variant (system under test). */
export type Variant = "rag" | "pageindex" | "pi-retriever";
export const ALL_VARIANTS: Variant[] = ["rag", "pageindex", "pi-retriever"];

/** The gold answer for a case. `direct` = specs whose contract changes. */
export interface Gold {
  direct: string[];
  related: string[];
}

/** One eval case: a task description + the specs it should surface. */
export interface Case {
  id: string;
  task: string;
  /** Optional file paths hint (mirrors `sdd context --paths`). */
  paths?: string[];
  gold: Gold;
  /** Originating archived change name, for traceability. */
  source?: string;
  /** Free-form note for human reviewers editing cases.json. */
  note?: string;
}

/** One (variant, case) run result. */
export interface CaseRun {
  variant: Variant;
  caseId: string;
  /** 0-indexed repeat number (stability probing). */
  repeat: number;
  result: RetrievalResult;
  latencyMs: number;
}

/** Computed metrics for a single CaseRun against its gold. */
export interface CaseMetrics {
  variant: Variant;
  caseId: string;
  repeat: number;
  unavailable: boolean;
  /** precision = |pred∩gold_direct| / |pred_direct| (0 if pred empty). */
  directPrecision: number;
  /** recall = |pred∩gold_direct| / |gold_direct| (0 if gold empty). */
  directRecall: number;
  directF1: number;
  /** 1 iff predicted direct set == gold direct set exactly. */
  exactDirectMatch: number;
  /** 1 iff every gold.direct appears in pred direct∪related (tolerate tier slips). */
  anyTierRecall: number;
  toolCalls?: number;
  truncated: boolean;
  latencyMs: number;
  error?: string;
}

/** Aggregated metrics for one variant across all cases. */
export interface VariantSummary {
  variant: Variant;
  n: number;
  /** number of (case,repeat) cells. */
  cells: number;
  unavailableRate: number;
  truncatedRate: number;
  meanDirectPrecision: number;
  meanDirectRecall: number;
  meanDirectF1: number;
  exactDirectMatchRate: number;
  anyTierRecallRate: number;
  meanToolCalls: number | null;
  meanLatencyMs: number;
  /** Only for non-deterministic variants: fraction of cases whose direct set
   *  was identical across all repeats. null when repeat <= 1. */
  stability: number | null;
}
