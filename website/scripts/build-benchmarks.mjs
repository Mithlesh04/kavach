// Writes data/benchmarks.json from the benchmark READMEs, where every
// measured run is recorded (decision 0033: never type a benchmark number
// into the site). data/benchmark-sources.json says which table row and
// column feed each chart; this script finds them and fails loudly when a
// row or a number is missing, so a renamed benchmark can't silently
// leave a stale chart behind.

import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const repo = path.join(root, "..");
const sources = JSON.parse(fs.readFileSync(path.join(root, "data", "benchmark-sources.json"), "utf8"));

/** Every Markdown table in a file: its header cells and its rows of cells. */
function tables(markdown) {
  const found = [];
  let current = null;
  for (const line of markdown.split(/\r?\n/)) {
    if (!line.startsWith("|")) {
      current = null;
      continue;
    }
    const cells = line.slice(1, line.endsWith("|") ? -1 : undefined).split("|").map((c) => c.trim());
    if (!current) {
      current = { header: cells, rows: [] };
      found.push(current);
    } else if (!cells.every((c) => /^:?-+:?$/.test(c))) {
      current.rows.push(cells);
    }
  }
  return found;
}

/** The number in the first table that has this row and column. */
function lookup(all, row, column, where) {
  for (const t of all) {
    const col = t.header.indexOf(column);
    if (col < 0) continue;
    const r = t.rows.find((cells) => cells[0] === row);
    if (!r) continue;
    const value = Number(r[col].replace(/\*\*/g, "").replace(/[,_\s]/g, ""));
    if (!Number.isFinite(value)) throw new Error(`${where}: "${r[col]}" in row ${row}, column ${column} is not a number`);
    return value;
  }
  throw new Error(`${where}: no table has row ${row} with column "${column}"`);
}

const out = {
  _generated_from:
    "the benchmark READMEs, by website/scripts/build-benchmarks.mjs — never edit by hand; record new runs in the README and run npm run check",
  suites: {},
};

// Suites whose README has a date only on a section their charts don't read.
const UNDATED = new Set(["gpu"]);

for (const [suiteId, suite] of Object.entries(sources.suites)) {
  const readmePath = path.join(repo, suite.readme);
  const markdown = fs.readFileSync(readmePath, "utf8");
  const all = tables(markdown);
  const measured = UNDATED.has(suiteId) ? null : (markdown.match(/Measured (\d{4}-\d{2}-\d{2})/)?.[1] ?? null);
  const benchmarks = {};
  for (const [benchId, bench] of Object.entries(suite.benchmarks)) {
    const where = `${suite.readme} (${suiteId}/${benchId})`;
    benchmarks[benchId] = {
      title: bench.title,
      unit: "ms",
      series: suite.series.map((s) => ({
        id: s.id,
        label: s.label,
        color: s.color,
        value: lookup(all, s.row ?? bench.row, bench.column ?? s.column, where),
      })),
      verify: suite.verify,
      readme: suite.readme,
    };
  }
  out.suites[suiteId] = { title: suite.title, environment: suite.environment, measured, benchmarks };
}

fs.writeFileSync(path.join(root, "data", "benchmarks.json"), `${JSON.stringify(out, null, 2)}\n`);
const count = Object.values(out.suites).reduce((n, s) => n + Object.keys(s.benchmarks).length, 0);
console.log(`build-benchmarks: ${count} benchmarks read from their READMEs`);
