import fs from "node:fs";
import path from "node:path";

// Reference pages read the spec itself, at build time, instead of a typed
// copy that could drift: the tables and code blocks of one spec section.
// The spec is the source of truth (AGENTS.md), and some tables are also
// held to the code by tests (spec 10.11's settings, by
// compiler/driver/tests/settings.rs).

/** `caption`: the line just before the table when it ends with ":", as "Text (`String`):" does in 14.2. */
export type SpecTable = { heading: string; header: string[]; rows: string[][]; caption?: string };

function section(file: string, number: string): string {
  const text = fs.readFileSync(path.join(process.cwd(), "..", "docs", "spec", file), "utf8").replace(/\r\n/g, "\n");
  if (text.startsWith(`# ${number} `)) return text; // a chapter split into files: one file per section
  const start = text.indexOf(`\n## ${number} `);
  if (start < 0) throw new Error(`spec ${file}: no section ${number}`);
  const end = text.indexOf("\n## ", start + 1);
  return text.slice(start, end < 0 ? undefined : end);
}

const cellsOf = (line: string) =>
  line
    .slice(1, line.endsWith("|") ? -1 : undefined)
    .split("|")
    .map((c) => c.trim());

/** Every Markdown table in a text, indented in a list item or not, with the `##` or `###` heading it sits under ("" before the first). */
function tablesIn(text: string, where: string): SpecTable[] {
  const found: SpecTable[] = [];
  let heading = "";
  let current: SpecTable | null = null;
  let lastText = "";
  for (const line of text.split("\n").map((l) => l.trimStart())) {
    const h = line.match(/^#{2,3} (.*)/);
    if (h) heading = h[1].trim();
    if (!line.startsWith("|")) {
      current = null;
      if (line.trim()) lastText = h ? "" : line.trim();
      continue;
    }
    if (!current) {
      current = { heading, header: cellsOf(line), rows: [], caption: lastText.endsWith(":") ? lastText : undefined };
      lastText = "";
      found.push(current);
    } else if (!cellsOf(line).every((c) => /^:?-+:?$/.test(c))) {
      current.rows.push(cellsOf(line));
    }
  }
  if (!found.length) throw new Error(`${where}: no tables`);
  return found;
}

/** Every Markdown table in a spec section. */
export const specTables = (file: string, number: string): SpecTable[] => tablesIn(section(file, number), `spec ${file} ${number}`);

/** Every Markdown table in a file of the repository (a decision record, a README). */
export const fileTables = (rel: string): SpecTable[] =>
  tablesIn(fs.readFileSync(path.join(process.cwd(), "..", rel), "utf8").replace(/\r\n/g, "\n"), rel);

/** The first capture of `pattern` in a file of the repository: a figure the file gives in a sentence, not a table. */
export function fileFigure(rel: string, pattern: RegExp): string {
  const found = fs.readFileSync(path.join(process.cwd(), "..", rel), "utf8").match(pattern);
  if (!found) throw new Error(`${rel}: nothing matches ${pattern}`);
  return found[1];
}

/** A fenced code block in a spec section: the first, or the one at `index`. */
/** The code block of a spec section that holds `text`, as written: a block indented under a bullet loses its indent. */
export function specBlockWith(file: string, number: string, text: string): string {
  for (const [, indent, code] of section(file, number).matchAll(/^( *)```[a-z]*\r?\n([\s\S]*?)\r?\n\1```/gm)) {
    if (code.includes(text)) return code.replace(new RegExp(`^${indent}`, "gm"), "");
  }
  throw new Error(`spec ${file} ${number}: no code block holds ${JSON.stringify(text)}`);
}

export function specBlock(file: string, number: string, index = 0): string {
  const blocks = [...section(file, number).matchAll(/```[a-z]*\n([\s\S]*?)\n```/g)];
  if (!blocks[index]) throw new Error(`spec ${file} ${number}: no code block ${index}`);
  return blocks[index][1];
}
