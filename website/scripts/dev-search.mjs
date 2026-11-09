// Search in `next dev`: Pagefind indexes the exported site, which `next dev`
// never writes, so the search box would find nothing. Before the dev server
// starts, this indexes the last built site (out/) into public/pagefind-dev/,
// which `next dev` serves and the search box loads in development. With no
// built site yet, it builds one first, once. The index is as fresh as that
// build: run `npm run build` again to search new pages.
//
// public/pagefind-dev/ is git-ignored, and `npm run build` removes it from
// out/, so it never ships.

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const out = path.join(root, "out");
const devIndex = path.join(root, "public", "pagefind-dev");

if (!fs.existsSync(path.join(out, "docs"))) {
  console.log("dev-search: no built site yet, so building it once for the search index (a few minutes)…");
  execSync("npx next build", { stdio: "inherit" });
}

fs.rmSync(devIndex, { recursive: true, force: true });
execSync(`npx pagefind --site out --output-path "${devIndex}"`, { stdio: ["ignore", "ignore", "inherit"] });
const built = fs.statSync(path.join(out, "index.html")).mtime.toLocaleString();
console.log(`dev-search: search works in dev, with the pages as built on ${built}`);
