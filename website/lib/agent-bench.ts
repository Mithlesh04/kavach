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

export type Way = { way: string; calls: number; tokensIn: number; tokensOut: number; seconds: number; right: string };
/** `notes` are the README's paragraphs between the case's table and the next case. */
export type Case = { name: string; title: string; about: string; data: string; ways: Way[]; notes: string[] };

const num = (s: string) => Number(s.replace(/[,\s]|s$/g, ""));

/** Every case, with what it does and each way's measured result. */
export function agentCases(): Case[] {
  const about = tables(section("The cases"))[0].slice(1);
  const lines = section("Results");
  const found: Case[] = [];
  lines.forEach((line, i) => {
    const title = line.match(/^\*\*(.+)\*\*$/)?.[1];
    if (!title) return;
    const rest = lines.slice(i + 1);
    const [table] = tables(rest);
    const [header, ...rows] = table;
    const next = rest.findIndex((l) => /^\*\*(.+)\*\*$/.test(l));
    const notes = rest
      .slice(0, next < 0 ? undefined : next)
      .filter((l) => l.trim() && !l.startsWith("|") && !l.startsWith("#"))
      .map(plain);
    if (header.join("|") !== "Way|Calls|Input tokens|Output tokens|Time|Right") {
      throw new Error(`benchmarks/agents/README.md: the table of "${title}" changed shape`);
    }
    const name = title.split(":")[0];
    const row = about.find((r) => r[0] === name);
    if (!row) throw new Error(`benchmarks/agents/README.md: "${name}" isn't in The cases`);
    found.push({
      name,
      title,
      about: plain(row[1]),
      data: plain(row[2]),
      ways: rows.map(([way, calls, tin, tout, time, right]) => ({
        way,
        calls: num(calls),
        tokensIn: num(tin),
        tokensOut: num(tout),
        seconds: num(time),
        right,
      })),
      notes,
    });
  });
  if (!found.length) throw new Error("benchmarks/agents/README.md: no results");
  return found;
}

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
