// BDD runner — Gherkin (.feature) → bun:test bridge.
//
// Minimal, dependency-free port of the llman-sdd 0.5 BDD runner pattern
// (MIT, github.com/StrayDragon/llman-sdd): parses the strict Gherkin subset
// emitted by `llman-sdd spec migrate-native` — no tables, no doc strings —
// so `bun test tests/bdd` works without package.json / node_modules.
//
//   - `# language: zh-CN` header; keywords 功能/规则/场景 (en fallbacks).
//   - Steps 假如/当/那么 map to given/when/then; 而且 repeats the last kind.
//   - Rule-nested and feature-level scenarios both execute.
//   - Step patterns support "{name}" (non-space), "{name:d}" (integer) and
//     "{name:rest}" (greedy remainder) placeholders.
//   - A per-scenario TestContext carries a fixture store (Given-produced
//     entities referenced by later steps).
//   - @skip / @experimental tags skip (harness convention; spec tags stay
//     inert per llman-sdd 0.5 — only this bridge reads them).

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

export type StepKind = 'given' | 'when' | 'then';

/** Per-scenario mutable state shared across steps. */
export interface TestContext {
  fixtures: Record<string, unknown>;
}

export interface StepDef {
  kind: StepKind;
  pattern: string;
  regex: RegExp;
  paramNames: string[];
  fn: (ctx: TestContext, params: Record<string, string>) => void | Promise<void>;
}

interface ParsedStep {
  kind: StepKind;
  text: string;
}

interface ParsedScenario {
  name: string;
  tags: string[];
  steps: ParsedStep[];
}

interface ParsedFeature {
  name: string;
  scenarios: ParsedScenario[];
}

// ---------------------------------------------------------------------------
// Step registry
// ---------------------------------------------------------------------------

const registry: StepDef[] = [];

function placeholderRegex(pattern: string): { regex: RegExp; paramNames: string[] } {
  const paramNames: string[] = [];
  const source = pattern
    .trim()
    .replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    .replace(/\\\{(\w+)(?::(d|rest))?\\\}/g, (_, name, type) => {
      paramNames.push(name);
      return type === 'd' ? '(\\d+)' : type === 'rest' ? '(.+)' : '([^\\s]+)';
    });
  return { regex: new RegExp(`^${source}$`), paramNames };
}

function register(kind: StepKind, pattern: string, fn: StepDef['fn']): void {
  const { regex, paramNames } = placeholderRegex(pattern);
  registry.push({ kind, pattern, regex, paramNames, fn });
}

export const bdd = {
  given: (pattern: string, fn: StepDef['fn']) => register('given', pattern, fn),
  when: (pattern: string, fn: StepDef['fn']) => register('when', pattern, fn),
  then: (pattern: string, fn: StepDef['fn']) => register('then', pattern, fn),
};

function resolveStep(kind: StepKind, text: string): { def: StepDef; params: Record<string, string> } {
  // Longest pattern wins: a literal step registered for one feature beats the
  // generic placeholder it would otherwise shadow, regardless of import order.
  const sameKind = registry.filter((d) => d.kind === kind).sort((a, b) => b.pattern.length - a.pattern.length);
  for (const def of sameKind) {
    const match = def.regex.exec(text);
    if (match) {
      const params: Record<string, string> = {};
      def.paramNames.forEach((name, i) => (params[name] = match[i + 1] ?? ''));
      return { def, params };
    }
  }
  const candidates = sameKind
    .map((d) => `  ${d.pattern}`)
    .join('\n');
  throw new Error(`no ${kind} step definition for: "${text}"\nregistered ${kind} patterns:\n${candidates}`);
}

// ---------------------------------------------------------------------------
// Parser (strict subset emitted by llman-sdd migrate-native)
// ---------------------------------------------------------------------------

const KEYWORDS: Record<string, { feature: string; rule: string; scenario: string; given: string[]; when: string[]; then: string[]; and: string[] }> = {
  'zh-CN': { feature: '功能:', rule: '规则:', scenario: '场景:', given: ['假如', '假设', '假定'], when: ['当'], then: ['那么'], and: ['而且', '并且', '同时'] },
  en: { feature: 'Feature:', rule: 'Rule:', scenario: 'Scenario:', given: ['Given'], when: ['When'], then: ['Then'], and: ['And'] },
};

function keywordsFor(text: string) {
  const language = /^# language:\s*(\S+)/m.exec(text)?.[1] ?? 'en';
  return KEYWORDS[language] ?? KEYWORDS.en;
}

function parseFeature(path: string): ParsedFeature {
  const text = readFileSync(path, 'utf8');
  const kw = keywordsFor(text);
  const feature: ParsedFeature = { name: '', scenarios: [] };
  let scenario: ParsedScenario | null = null;
  let pendingTags: string[] = [];
  let lastKind: StepKind = 'given';

  const stepKind = (line: string): StepKind | null => {
    if (kw.given.some((k) => line.startsWith(k + ' '))) return 'given';
    if (kw.when.some((k) => line.startsWith(k + ' '))) return 'when';
    if (kw.then.some((k) => line.startsWith(k + ' '))) return 'then';
    if (kw.and.some((k) => line.startsWith(k + ' '))) return lastKind;
    return null;
  };

  for (const raw of text.split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#') || line.startsWith('"""') || line.startsWith('|')) continue;
    if (line.startsWith('@')) {
      pendingTags.push(line);
      continue;
    }
    if (line.startsWith(kw.feature)) {
      feature.name = line.slice(kw.feature.length).trim();
    } else if (line.startsWith(kw.rule)) {
      scenario = null; // a rule description ends any pending step block
      pendingTags = [];
    } else if (line.startsWith(kw.scenario)) {
      scenario = { name: line.slice(kw.scenario.length).trim(), tags: pendingTags, steps: [] };
      pendingTags = [];
      feature.scenarios.push(scenario);
    } else {
      const kind = stepKind(line);
      if (kind && scenario) {
        const keyword = kw.given.concat(kw.when, kw.then, kw.and).find((k) => line.startsWith(k))!;
        scenario.steps.push({ kind, text: line.slice(keyword.length).trim() });
        lastKind = kind;
      }
      // Anything else (rule descriptions etc.) is prose — ignored.
    }
  }
  return feature;
}

// ---------------------------------------------------------------------------
// bun:test bridge
// ---------------------------------------------------------------------------

export function runFeature(
  path: string,
  makeContext: () => TestContext,
  include?: RegExp,
  skipIf?: () => boolean,
): void {
  const feature = parseFeature(path);
  const t = skipIf?.() ? test.skip : test;
  describe(feature.name || path, () => {
    for (const scenario of feature.scenarios) {
      if (include && !include.test(scenario.name)) continue;
      if (scenario.tags.some((t) => /@skip|@experimental/.test(t))) continue;
      t(scenario.name, async () => {
        const ctx = makeContext();
        for (const step of scenario.steps) {
          const { def, params } = resolveStep(step.kind, step.text);
          await def.fn(ctx, params);
        }
        expect(scenario.steps.length).toBeGreaterThan(0);
      });
    }
  });
}
