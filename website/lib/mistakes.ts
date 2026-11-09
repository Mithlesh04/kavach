import fs from "node:fs";
import path from "node:path";
import { fileTables } from "./spec-tables";

// The mistakes benchmark (benchmarks/mistakes/), read at build time: each
// everyday mistake, the step that catches it, and the message. The site
// check runs every case again and fails if a row no longer matches
// (scripts/check-samples.mjs), so the page can't drift from the compiler.

const README = "benchmarks/mistakes/README.md";
const dir = path.join(process.cwd(), "..", "benchmarks", "mistakes");

export type Mistake = { title: string; file: string; step: string; message: string; code: string };

/** Every mistake, in the README's order, with its case's code below the title line. */
export function mistakes(): Mistake[] {
  const t = fileTables(README).find((x) => x.header.join("|") === "Mistake|Caught by|Message");
  if (!t) throw new Error(`${README}: no Mistake|Caught by|Message table`);
  return t.rows.map(([mistake, step, message]) => {
    const m = mistake.match(/^\[(.+)\]\(cases\/([^)]+)\)$/);
    if (!m) throw new Error(`${README}: "${mistake}" doesn't link its case`);
    const text = fs.readFileSync(path.join(dir, "cases", m[2]), "utf8").replace(/\r\n/g, "\n");
    return {
      title: m[1],
      file: m[2],
      step,
      message: message.replace(/\\\|/g, "|"),
      code: text.split("\n").slice(1).join("\n").trim(),
    };
  });
}

/** The steps a mistake can be caught at, as the README explains them. */
export function mistakeSteps(): { step: string; means: string }[] {
  const t = fileTables(README).find((x) => x.header.join("|") === "Step|What it means");
  if (!t) throw new Error(`${README}: no Step|What it means table`);
  return t.rows.map(([step, means]) => ({ step, means }));
}
