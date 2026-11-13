// What every standard-library file declares, for the Reference
// (content/lessons/reference-std.tsx): `kavach outline` gives each file's
// declarations without their bodies, with their doc comments, contracts
// and `example`s, which `kavach test` checks (spec 7.6). Reading them
// from the compiler, not from a typed list, keeps the Reference in step
// with stdlib/.
//
// This writes data/outlines.json: each file's outline, keyed by its path
// from the repository root. The file is committed, so a machine without
// kavach (a plain web host) still builds the pages.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { findKavach } from "../lib/find-kavach.mjs";

const root = process.cwd();
const repo = path.join(root, "..");
const out = path.join(root, "data", "outlines.json");

const kavach = findKavach(root);
if (!kavach) {
  console.warn("build-outlines: no kavach binary found (set KAVACH_BIN): data/outlines.json kept as it is");
  process.exit(0);
}

/** Every .kav file of stdlib/, its tests left out, in a steady order. */
function stdlibFiles(dir) {
  const found = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const p = path.join(dir, e.name);
    if (e.isDirectory() && e.name !== "tests") found.push(...stdlibFiles(p));
    else if (e.isFile() && e.name.endsWith(".kav")) found.push(p);
  }
  return found;
}

const outlines = {};
for (const file of stdlibFiles(path.join(repo, "stdlib"))) {
  const rel = path.relative(repo, file).replace(/\\/g, "/");
  try {
    // The outline is on standard output; how much it saved goes to standard error.
    outlines[rel] = execFileSync(kavach, ["outline", file], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] })
      .replace(/\r\n/g, "\n")
      .trimEnd();
  } catch (e) {
    console.error(`build-outlines: \`kavach outline ${rel}\` failed\n${e.stderr ?? e.message}`);
    process.exit(1);
  }
}

fs.writeFileSync(out, `${JSON.stringify(outlines, null, 1)}\n`);
console.log(`build-outlines: ${Object.keys(outlines).length} standard-library files outlined`);
