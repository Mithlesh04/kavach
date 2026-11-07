// The reference guard (decision 0033): every <SrcRef> into our own
// repository must point at a file that exists, and its symbol must
// still live within a few lines of the stated line — so "check the
// source" links can never rot. References into other projects' repos
// (repo={...}) are not ours to verify and are skipped.

import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const repo = path.join(root, "..");
const contentDir = path.join(root, "content");
const DRIFT = 40; // lines the symbol may sit from the stated line

function* tsxFiles(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* tsxFiles(p);
    else if (/\.(tsx|ts|mdx)$/.test(entry.name)) yield p;
  }
}

let failed = 0;
let checked = 0;
for (const file of tsxFiles(contentDir)) {
  const src = fs.readFileSync(file, "utf8");
  const rel = path.relative(root, file);
  for (const m of src.matchAll(/<SrcRef\b[^>]*?\/>/gs)) {
    const tag = m[0];
    if (tag.includes("repo={")) continue; // someone else's repository
    const fileAttr = tag.match(/file="([^"]+)"/)?.[1];
    if (!fileAttr) continue;
    checked++;
    const target = path.join(repo, fileAttr);
    if (!fs.existsSync(target)) {
      console.error(`GONE      ${rel} -> ${fileAttr} (no such file in the repository)`);
      failed++;
      continue;
    }
    const line = Number(tag.match(/line=\{(\d+)\}/)?.[1] ?? 0);
    const symbol = tag.match(/symbol="([^"]+)"/)?.[1];
    if (line && symbol) {
      const lines = fs.readFileSync(target, "utf8").split(/\r?\n/);
      const lo = Math.max(0, line - 1 - DRIFT);
      const hi = Math.min(lines.length, line - 1 + DRIFT);
      const near = lines.slice(lo, hi).some((l) => l.includes(symbol));
      if (!near) {
        console.error(
          `MOVED     ${rel} -> ${fileAttr}:${line} ${symbol}() — not within ${DRIFT} lines; update the reference`,
        );
        failed++;
      }
    }
  }
}

if (failed) {
  console.error(`check-refs: ${failed} of ${checked} source references rot — fix them before shipping docs`);
  process.exit(1);
}
console.log(`check-refs: ${checked} source references verified against the working tree`);
