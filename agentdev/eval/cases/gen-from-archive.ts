// Generate eval cases (gold answers) from archived SDD changes.
//
// Each archived change is a free label: its proposal describes the task, and the
// `specs/<id>/` deltas name the specs whose contract changed — exactly what
// `direct` should surface. Output is a human-editable cases.json.
//
//   llmanspec/changes/archive/<change>/proposal.md   -> task title + why snippet
//   llmanspec/changes/archive/<change>/specs/<id>/   -> gold.direct (contract changed)
//
// Changes with NO specs/ delta (e.g. they only created a brand-new spec) are
// written to needs-gold.json for manual labelling rather than silently dropped.
import { readFile, readdir, stat, writeFile, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import type { Case, Gold } from "../lib/types.ts";

function stripFrontmatter(md: string): string {
  if (!md.startsWith("---")) return md;
  const end = md.indexOf("\n---", 3);
  if (end === -1) return md;
  return md.slice(end + 4).replace(/^\s*\n/, "");
}

/** First non-empty H1 line, cleaned of "# " and optional "Proposal: " prefix. */
function extractTitle(md: string): string {
  const body = stripFrontmatter(md);
  for (const line of body.split("\n")) {
    const m = line.match(/^#\s+(.*)$/);
    if (m) {
      return m[1].replace(/^Proposal:\s*/i, "").trim();
    }
  }
  return "";
}

/** First prose sentence under the first "## Why" heading, for human context. */
function extractWhySnippet(md: string): string {
  const body = stripFrontmatter(md);
  const whyIdx = body.search(/^##\s*Why\b/im);
  if (whyIdx === -1) return "";
  const after = body.slice(whyIdx);
  // skip the heading line, then take the first non-empty, non-heading paragraph
  const lines = after.split("\n").slice(1);
  let para = "";
  for (const ln of lines) {
    if (/^#/.test(ln)) break;
    if (ln.trim()) {
      para += (para ? " " : "") + ln.trim();
      if (/[.。!?]$/.test(para)) break;
    } else if (para) break;
  }
  return para.slice(0, 280);
}

async function isDir(p: string): Promise<boolean> {
  try {
    return (await stat(p)).isDirectory();
  } catch {
    return false;
  }
}

async function readChange(dir: string): Promise<Case | null> {
  const name = dir.split("/").pop()!.replace(/^\d+-/, "");
  const proposalPath = join(dir, "proposal.md");
  let proposal = "";
  try {
    proposal = await readFile(proposalPath, "utf8");
  } catch {
    return null;
  }
  // Prefer the proposal's H1 title (most natural task wording); fall back to
  // the change dir name (sans date prefix) when a proposal opens straight into
  // `## Why` with no H1 — still a valid, if terser, task description.
  const title = extractTitle(proposal) || name;

  const specsDir = join(dir, "specs");
  const goldDirect: string[] = [];
  if (await isDir(specsDir)) {
    for (const entry of await readdir(specsDir)) {
      if (entry.startsWith(".")) continue;
      if (await isDir(join(specsDir, entry))) goldDirect.push(entry);
    }
  }
  return {
    id: name,
    task: title,
    gold: { direct: goldDirect.sort(), related: [] },
    source: dir.split("/").pop()!,
    note: extractWhySnippet(proposal),
  };
}

export async function generateCases(projectDir: string, outPath: string) {
  // projectDir is the project ROOT (contains llmanspec/), not llmanspec/ itself —
  // this matches how `llman sdd context` discovers the spec dir from cwd.
  const archiveDir = join(projectDir, "llmanspec", "changes", "archive");
  let entries: string[] = [];
  try {
    entries = (await readdir(archiveDir)).filter((e) => !e.startsWith("."));
  } catch (e) {
    throw new Error(`cannot read archive at ${archiveDir}: ${String(e)}`);
  }
  entries.sort();

  const cases: Case[] = [];
  const needsGold: Case[] = [];
  for (const entry of entries) {
    const dir = join(archiveDir, entry);
    if (!(await isDir(dir))) continue;
    const c = await readChange(dir);
    if (!c) continue;
    if (c.gold.direct.length === 0) {
      needsGold.push(c);
    } else {
      cases.push(c);
    }
  }

  const absOut = resolve(outPath);
  await mkdir(dirname(absOut), { recursive: true });
  await writeFile(absOut, JSON.stringify(cases, null, 2) + "\n", "utf8");
  const needsPath = absOut.replace(/\.json$/, ".needs-gold.json");
  if (needsGold.length) {
    await writeFile(needsPath, JSON.stringify(needsGold, null, 2) + "\n", "utf8");
  }
  return { cases, needsGold };
}

// ---- CLI ----
async function main() {
  const args = process.argv.slice(2);
  const projectIdx = args.indexOf("--project");
  const projectDir = projectIdx >= 0 ? args[projectIdx + 1] : "../..";
  const outIdx = args.indexOf("--out");
  const outPath = outIdx >= 0 ? args[outIdx + 1] : "cases.json";
  const { cases, needsGold } = await generateCases(projectDir, outPath);
  console.log(`generated ${cases.length} cases -> ${outPath}`);
  for (const c of cases) {
    console.log(
      `  [${c.id}] direct={${c.gold.direct.join(",")}}  task=${JSON.stringify(c.task).slice(0, 70)}`,
    );
  }
  if (needsGold.length) {
    console.log(
      `\n${needsGold.length} change(s) have no specs/ delta (need manual gold) -> ${outPath.replace(/\.json$/, ".needs-gold.json")}`,
    );
    for (const c of needsGold) console.log(`  [${c.id}]`);
  }
}

if (import.meta.path === Bun.main) {
  main();
}
