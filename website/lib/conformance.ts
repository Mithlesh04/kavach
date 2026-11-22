import fs from "node:fs";
import path from "node:path";

// Every conformance test (tests/conformance/), read from its header when the
// site is built (format: tests/conformance/README.md#test-format). Nothing is
// typed by hand, so a new test shows up on the next build, under the spec
// section its `// Spec:` line names first.

const repo = path.join(process.cwd(), "..");
const ROOT = "tests/conformance";

export type ConformanceTest = {
  /** The file with the header, from the repo root: a `.kav` file, or a project's `src/main.kav`. */
  file: string;
  /** The test's name: its file's, or its project folder's. */
  name: string;
  /** The rule it checks, as its `// Rule:` line says. */
  rule: string;
  /** The sections its `// Spec:` line names; the first is the one it is listed under. */
  sections: string[];
  /** What it expects: `runs`, or how wrong use is refused. */
  outcome: string;
  /** Whether it checks that wrong use is refused, not that something works. */
  refused: boolean;
};

/** A header's `// key: value` lines, and its bare `// expect …` lines. */
function header(text: string): { fields: Map<string, string[]>; flags: Set<string> } {
  const fields = new Map<string, string[]>();
  const flags = new Set<string>();
  // A test may start with a byte order mark on purpose (01-basics/a-byte-order-mark-is-a-space.kav).
  for (const line of text.replace(/^﻿/, "").replace(/\r\n/g, "\n").split("\n")) {
    if (!line.startsWith("//")) break;
    const body = line.slice(2).trim();
    const field = body.match(/^(Rule|Spec|Checks|expect [a-z ]+?|args):\s*(.*)$/);
    if (field) fields.set(field[1], [...(fields.get(field[1]) ?? []), field[2].trim()]);
    else if (body.startsWith("expect ")) flags.add(body);
  }
  return { fields, flags };
}

/** How a test's expectations read on the site: refused ones say how. */
function outcomeOf(fields: Map<string, string[]>, flags: Set<string>): string {
  const exit = Number(fields.get("expect exit")?.[0] ?? "0");
  if (flags.has("expect compile error")) return "compile error";
  if (flags.has("expect test fail")) return "test fails";
  if (flags.has("expect panic") || fields.has("expect panic")) return "panic";
  if (exit !== 0) return `exit ${exit}`;
  if (flags.has("expect warning")) return "warning";
  if (fields.get("Checks")?.includes("misuse")) return "error value";
  return "runs";
}

/** The test files under a folder: a project folder is one test (as tools/cli/tests/conformance.rs finds them). */
function testFiles(rel: string): string[] {
  const dir = path.join(repo, rel);
  if (fs.existsSync(path.join(dir, "kavach.toml")) && fs.existsSync(path.join(dir, "src"))) return [`${rel}/src/main.kav`];
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((e) => (e.isDirectory() ? testFiles(`${rel}/${e.name}`) : e.name.endsWith(".kav") ? [`${rel}/${e.name}`] : []));
}

function read(file: string): ConformanceTest {
  const { fields, flags } = header(fs.readFileSync(path.join(repo, file), "utf8"));
  const rule = fields.get("Rule")?.[0];
  const spec = fields.get("Spec")?.[0];
  if (!rule || !spec) throw new Error(`${file}: a conformance test needs // Rule: and // Spec: (tests/conformance/README.md)`);
  const sections = spec.split(/[,\s]+/).filter((s) => /^\d+(\.\d+)*$/.test(s));
  if (!sections.length) throw new Error(`${file}: // Spec: names no section: ${spec}`);
  const outcome = outcomeOf(fields, flags);
  const name = file.endsWith("/src/main.kav") ? file.split("/").at(-3)! : path.basename(file, ".kav");
  return { file, name, rule, sections, outcome, refused: outcome !== "runs" };
}

/** Every conformance test, in folder order. */
export const conformanceTests: ConformanceTest[] = testFiles(ROOT).map(read);

/** The tests listed under a section: those whose first section is it, or inside it (`14.11` holds `14.11.2`). */
export const testsOf = (number: string): ConformanceTest[] =>
  conformanceTests.filter((t) => t.sections[0] === number || t.sections[0].startsWith(`${number}.`));

/** Every spec heading's number and title: `1.4` → `Names`, `14.3` → `std.fs`, `6` → `Memory`. */
export const sectionTitles: Map<string, string> = (() => {
  const titles = new Map<string, string>();
  const walk = (dir: string) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith(".md")) {
        for (const m of fs.readFileSync(full, "utf8").matchAll(/^#{1,4} (\d+(?:\.\d+)*)\.? (.+)$/gm)) {
          if (!titles.has(m[1])) titles.set(m[1], m[2].trim());
        }
      }
    }
  };
  walk(path.join(repo, "docs", "spec"));
  return titles;
})();

/**
 * A conformance test as a page shows it: its program, without its header,
 * and what it prints, read from its `// expect output:` lines, then its
 * `// expect stderr:` lines. For what a sample's .expect can't hold, such
 * as the `at` line a panic prints under its message. The program is shown
 * as `file`, so a place the test expects (`at <test>.kav:11`) is
 * renumbered to the line shown and named as `file`: the same line of the
 * same code.
 */
export function conformanceRun(rel: string, file = "main.kav"): { code: string; output: string } {
  const lines = fs.readFileSync(path.join(repo, rel), "utf8").replace(/\r\n/g, "\n").split("\n");
  const body = lines.findIndex((l) => !l.startsWith("//") && l.trim() !== "");
  if (body < 0) throw new Error(`${rel}: no program after its header`);
  const said = (kind: string) =>
    lines.slice(0, body).filter((l) => l.startsWith(`// expect ${kind}: `)).map((l) => l.slice(`// expect ${kind}: `.length));
  const name = path.basename(rel).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const output = [...said("output"), ...said("stderr")].map((l) =>
    l.replace(new RegExp(`${name}:(\\d+)`, "g"), (_, n: string) => {
      const shown = Number(n) - body;
      if (shown < 1) throw new Error(`${rel}: expects a place in its own header, line ${n}`);
      return `${file}:${shown}`;
    }),
  );
  if (!output.length) throw new Error(`${rel}: its header expects no output`);
  return { code: lines.slice(body).join("\n").trimEnd(), output: output.join("\n") };
}
