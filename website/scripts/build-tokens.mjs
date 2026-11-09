// Names colored by where they come from (to-do item 5): the grammar can't
// tell an imported name or a built-in method from any other, so the
// compiler marks them, with `kavach query tokens` (spec 13.2), and the
// site's highlighter colors the marks (components/code-block.tsx).
//
// This asks once for every sample and writes data/tokens.json: the marks
// of each sample, keyed by the code the page shows. The file is committed,
// so a machine without kavach (a plain web host) still builds with them.
// With kavach but without the command, the check fails.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { findKavach } from "../lib/find-kavach.mjs";

const root = process.cwd();
const samplesDir = path.join(root, "content", "samples");
const out = path.join(root, "data", "tokens.json");

// The two ways a page shows a sample's file (lib/samples.ts): a project
// file as it is, and a one-file sample without the empty `fn main` a
// refused sample needs. Both keep every line and column of the rest.
const asProjectFile = (text) => text.replace(/\r\n/g, "\n").trimEnd();
const asSample = (text) => asProjectFile(text.replace(/\r\n/g, "\n").replace(/\n+fn main\(\) \{\n?\}\s*$/, ""));

const kavach = findKavach(root);
if (!kavach) {
  console.warn("build-tokens: no kavach binary found (set KAVACH_BIN): data/tokens.json kept as it is");
  process.exit(0);
}

let lines;
try {
  lines = execFileSync(kavach, ["query", "tokens", "--format", "json", samplesDir], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
} catch (e) {
  console.error(`build-tokens: \`kavach query tokens\` failed: is ${kavach} older than the command?\n${e.stderr ?? e.message}`);
  process.exit(1);
}

const marks = {};
let files = 0;
let skipped = 0;
for (const line of lines.split(/\r?\n/).filter(Boolean)) {
  const entry = JSON.parse(line);
  if (entry.error || !entry.tokens?.length) {
    skipped += entry.error ? 1 : 0;
    continue;
  }
  const file = path.resolve(root, entry.file);
  const text = fs.readFileSync(file, "utf8");
  // Only the marks that change a color: imported names and built-in methods.
  const kept = entry.tokens.filter(([, , , type, mods]) => mods.includes("imported") || (type === "method" && mods.includes("defaultLibrary")));
  if (!kept.length) continue;
  files++;
  for (const key of new Set([asProjectFile(text), asSample(text)])) marks[key] = kept;
}

fs.writeFileSync(out, `${JSON.stringify(marks)}\n`);
console.log(`build-tokens: marks for ${files} samples (${skipped} that don't compile keep the grammar's colors)`);
