import fs from "node:fs";
import path from "node:path";

// The agent benchmark (benchmarks/agents/README.md), read at build time:
// its cases, its ways, and its measured results. Numbers on the site come
// only from here, never typed into a page (decision 0033).

const dir = path.join(process.cwd(), "..", "benchmarks", "agents");
const readme = () => fs.readFileSync(path.join(dir, "README.md"), "utf8").replace(/\r\n/g, "\n");

const cells = (line: string) =>
  line
    .slice(1, line.endsWith("|") ? -1 : undefined)
    .split("|")
    .map((c) => c.trim());

/** The Markdown tables in the lines given, each as rows of cells, its header first. */
function tables(lines: string[]): string[][][] {
  const out: string[][][] = [];
  let current: string[][] | null = null;
  for (const line of lines) {
    if (!line.startsWith("|")) {
      current = null;
      continue;
    }
    if (!current) out.push((current = []));
    if (!cells(line).every((c) => /^:?-+:?$/.test(c))) current.push(cells(line));
  }
  return out;
}

function section(name: string): string[] {
  const lines = readme().split("\n");
  const start = lines.findIndex((l) => l === `## ${name}`);
  if (start < 0) throw new Error(`benchmarks/agents/README.md: no section ${name}`);
  const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
  return lines.slice(start + 1, end < 0 ? undefined : end);
}

/** `[text](url)` becomes its text; backticks stay for the page to show as code. */
export const plain = (cell: string) => cell.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1").replace(/\*\*/g, "");

/** `cost` is what the calls took at the provider's prices, for the cases measured with a Cost column. */
export type Way = { way: string; calls: number; tokensIn: number; tokensOut: number; seconds: number; right: string; cost?: string };
/** `notes` are the README's paragraphs between the case's table and the next case. `added` is the README's
 *  sentence for cases added later ("The two cases below were added on …"), when it stands above this one. */
export type Case = { name: string; title: string; about: string; data: string; ways: Way[]; notes: string[]; added?: string; model?: string };

const num = (s: string) => Number(s.replace(/[,\s]|s$/g, ""));
const CASE_COLUMNS = ["Way", "Calls", "Input tokens", "Output tokens", "Time", "Right"];
/** A paragraph that introduces the cases after it, not a note on the one before. */
const isIntro = (l: string) => /^The \w+ cases? below /.test(l);

type Block = { title: string; model?: string; header: string[]; rows: string[][]; paragraphs: string[] };

/** A case's bold title, with the model after it in brackets when the case names one: **Title** (`model`). */
const TITLE = /^\*\*(.+?)\*\*(?: \(`([^`]+)`\))?$/;

/** Each bold title of the Results section, with the first table and the paragraphs up to the next title. */
function resultBlocks(): Block[] {
  const lines = section("Results");
  const out: Block[] = [];
  lines.forEach((line, i) => {
    const [, title, model] = line.match(TITLE) ?? [];
    if (!title) return;
    const rest = lines.slice(i + 1);
    const next = rest.findIndex((l) => TITLE.test(l));
    const own = rest.slice(0, next < 0 ? undefined : next);
    const [table] = tables(own);
    if (!table) throw new Error(`benchmarks/agents/README.md: "${title}" has no table`);
    const [header, ...rows] = table;
    const paragraphs = own.filter((l) => l.trim() && !l.startsWith("|") && !l.startsWith("#"));
    out.push({ title, model, header, rows, paragraphs });
  });
  return out;
}

/** Every case, with what it does and each way's measured result. */
export function agentCases(): Case[] {
  const about = tables(section("The cases"))[0].slice(1);
  const found: Case[] = [];
  let added: string | undefined;
  for (const { title, model, header, rows, paragraphs } of resultBlocks()) {
    const name = title.split(":")[0];
    const row = about.find((r) => r[0] === name);
    if (!row) continue; // not a case of the same tasks, such as the crash test (agentCrash)
    const col = (c: string) => header.indexOf(c);
    const shape = header.filter((h) => h !== "Cost");
    if (shape.join("|") !== CASE_COLUMNS.join("|")) {
      throw new Error(`benchmarks/agents/README.md: the table of "${title}" changed shape`);
    }
    found.push({
      name,
      title,
      about: plain(row[1]),
      data: plain(row[2]),
      ways: rows.map((r) => ({
        way: r[col("Way")],
        calls: num(r[col("Calls")]),
        tokensIn: num(r[col("Input tokens")]),
        tokensOut: num(r[col("Output tokens")]),
        seconds: num(r[col("Time")]),
        right: r[col("Right")],
        cost: col("Cost") < 0 ? undefined : r[col("Cost")],
      })),
      notes: paragraphs.filter((p) => !isIntro(p)).map(plain),
      added,
      model,
    });
    const intro = paragraphs.find(isIntro);
    if (intro) added = plain(intro);
  }
  if (!found.length) throw new Error("benchmarks/agents/README.md: no results");
  return found;
}

/** The crash test: its title, its first paragraph (when and with which model), its table, and what it shows. */
export function agentCrash(): { title: string; when: string; header: string[]; rows: string[][]; notes: string[] } {
  const block = resultBlocks().find((b) => b.title.startsWith("A crash"));
  if (!block) throw new Error("benchmarks/agents/README.md: no crash test");
  if (block.header[0] !== "Way" || !block.header.includes("Paid twice")) {
    throw new Error("benchmarks/agents/README.md: the crash test's table changed shape");
  }
  const when = block.paragraphs[0].match(/^([\d-]+, `[^`]+`)\./)?.[1];
  if (!when) throw new Error("benchmarks/agents/README.md: the crash test no longer says when and with which model");
  return { title: block.title, when, header: block.header, rows: block.rows, notes: block.paragraphs.slice(1).map(plain) };
}

/** A bold-titled part of "Several models": its table, its paragraphs and its bullets, in the README's words. */
export type ModelBlock = { title: string; header: string[]; rows: string[][]; paragraphs: string[]; bullets: string[] };

const MODEL_WAYS = "Way|Cases|Right|Calls|Input tokens|Output tokens|Thinking|Cost|Time";
const shapeOf = (b: ModelBlock | undefined, header: string, what: string) => {
  if (!b || !b.header.join("|").startsWith(header)) throw new Error(`benchmarks/agents/README.md: ${what} changed shape`);
  return b;
};

/** The cases run on several models: the section's opening paragraphs, one table per model, the agentic loop's
 *  cases on each model, the crash test on each model, and how the answers moved from run to run. */
export function agentModels(): { intro: string[]; models: ModelBlock[]; loop: ModelBlock; crash: ModelBlock; runs: ModelBlock } {
  const lines = section("Several models");
  const first = lines.findIndex((l) => TITLE.test(l));
  // A "([how](method.md#…))" link would read as a bare "(how)": the page links method.md itself.
  const text = (ls: string[]) =>
    ls.filter((l) => l.trim() && !l.startsWith("|") && !l.startsWith("#") && !l.startsWith("- ")).map((l) => plain(l.replace(/ \(\[how\]\([^)]+\)\)/g, "")));
  const blocks: ModelBlock[] = [];
  lines.forEach((line, i) => {
    const title = line.match(TITLE)?.[1];
    if (!title) return;
    const rest = lines.slice(i + 1);
    const next = rest.findIndex((l) => TITLE.test(l));
    const own = rest.slice(0, next < 0 ? undefined : next);
    const [header = [], ...rows] = tables(own)[0] ?? [];
    blocks.push({ title, header, rows, paragraphs: text(own), bullets: own.filter((l) => l.startsWith("- ")).map((l) => l.slice(2)) });
  });
  const models = blocks.filter((b) => b.header.join("|") === MODEL_WAYS);
  if (first < 0 || !models.length) throw new Error("benchmarks/agents/README.md: Several models has no model tables");
  return {
    intro: text(lines.slice(0, first)),
    models,
    loop: shapeOf(blocks.find((b) => /^On the agentic tool loop's \d+ cases$/.test(b.title)), "Model|Way|Right|Tokens|Cost|Time", "the agentic loop's table"),
    crash: shapeOf(blocks.find((b) => b.title === "The crash test on each model"), "Model|Kavach: paid twice", "the crash test on each model"),
    runs: shapeOf(blocks.find((b) => b.title === "From run to run"), "Model|Way|Right in each run|Changed|Tokens in each run|Cost in each run", "From run to run"),
  };
}

/** A README section of one table, its sentences and its bullets, checked against its first column names. */
function sectionBlock(name: string, headerPrefix: string): ModelBlock {
  const lines = section(name);
  const [header = [], ...rows] = tables(lines)[0] ?? [];
  const b: ModelBlock = {
    title: name,
    header,
    rows,
    paragraphs: lines.filter((l) => l.trim() && !l.startsWith("|") && !l.startsWith("- ")).map((l) => plain(l.replace(/ \(\[how\]\([^)]+\)\)/g, ""))),
    bullets: lines.filter((l) => l.startsWith("- ")).map((l) => l.slice(2)),
  };
  return shapeOf(b, headerPrefix, name);
}

/** All twelve cases with a provider that fails on purpose. */
export const agentFaults = () => sectionBlock("When the provider fails", "Way|Right, no faults|Right at 30% faults|Lost");

/** The README's "Under attack": refund notes that try to steer the money, on each model. */
export const agentAttacks = () => sectionBlock("Under attack", "Model|Way|Right of 20|Paid wrongly|Tokens|Cost");

/** The ways compared, as the README describes them. */
export function agentWays(): { way: string; what: string }[] {
  return tables(section("The ways"))[0]
    .slice(1)
    .map(([way, what]) => ({ way, what: plain(what) }));
}

/** The README's run line, as it is. */
export function agentRunCommand(): string {
  const m = section("Running it").join("\n").match(/```\n([\s\S]*?)\n```/);
  if (!m) throw new Error("benchmarks/agents/README.md: no run command");
  return m[1];
}

/** A file of the benchmark, as it is: the code that was measured. */
export const agentFile = (rel: string) => fs.readFileSync(path.join(dir, rel), "utf8").replace(/\r\n/g, "\n").trimEnd();

/** A Python function of the benchmark, from its `def` to the next top-level line. */
export function pythonFunction(rel: string, name: string): string {
  const lines = agentFile(rel).split("\n");
  const start = lines.findIndex((l) => l.startsWith(`def ${name}(`));
  if (start < 0) throw new Error(`benchmarks/agents/${rel}: no function ${name}`);
  let end = start + 1;
  while (end < lines.length && (lines[end] === "" || lines[end].startsWith(" "))) end++;
  return lines.slice(start, end).join("\n").trimEnd();
}

export type Order = { id: number; amount: number; days: number; note: string; expect: string };

/** The large case's refund requests and their expected answers (data/orders.json). */
export const agentOrders = (): Order[] => JSON.parse(agentFile("data/orders.json"));
