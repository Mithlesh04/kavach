// The versioned multi-build (decision 0033): one static tree per docs
// version. `latest` is the ordinary build; every other entry in
// data/versions.json builds from its git tag into out/<id>/ with
// next.config's basePath set, so its pages and file:line links stay
// pinned to that tag forever. A version marked `"retired": true` gets
// only a stub that redirects to latest with a notice.
//
// Usage: node scripts/build-versions.mjs   (after `npm run build`)

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const repo = path.resolve(root, "..");
const versions = JSON.parse(fs.readFileSync(path.join(root, "data", "versions.json"), "utf8"));
const out = path.join(root, "out");

if (!fs.existsSync(out)) {
  console.error("build-versions: run `npm run build` first — out/ is missing");
  process.exit(1);
}

const run = (cmd, cwd) => execSync(cmd, { cwd, stdio: "pipe", encoding: "utf8" });

for (const v of versions.versions) {
  if (v.id === versions.latest) continue;

  if (v.retired) {
    const dir = path.join(out, v.id);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(
      path.join(dir, "index.html"),
      `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Kavach docs — ${v.id} retired</title><meta http-equiv="refresh" content="4;url=/"><style>body{font-family:system-ui;max-width:48ch;margin:20vh auto;padding:0 16px;color:#1c1917}</style></head><body><h1>Docs ${v.id} was retired</h1><p>You are being taken to the latest documentation. The code this version described is still on its git tag <code>${v.tag}</code>.</p><p><a href="/">Go now</a></p></body></html>`,
    );
    console.log(`build-versions: ${v.id} retired — redirect stub written`);
    continue;
  }

  // A frozen version builds from its tag, in a worktree inside the
  // repository's git-ignored .kavach/ folder (nothing outside the project).
  const wt = path.join(repo, ".kavach", "site-build", v.id);
  try {
    run(`git rev-parse --verify ${v.tag}`, repo);
  } catch {
    console.warn(`build-versions: tag ${v.tag} for ${v.id} does not exist yet — skipped`);
    continue;
  }
  fs.rmSync(wt, { recursive: true, force: true });
  run(`git worktree add --force "${wt}" ${v.tag}`, repo);
  try {
    const site = path.join(wt, "website");
    if (!fs.existsSync(site)) {
      console.warn(`build-versions: ${v.tag} has no website/ — docs begin later; skipped`);
      continue;
    }
    run("npm ci", site);
    // next.config.ts reads SITE_BASE_PATH, so this tree serves at /<id>/.
    execSync("npm run build", {
      cwd: site,
      stdio: "pipe",
      env: { ...process.env, SITE_BASE_PATH: `/${v.id}` },
    });
    fs.cpSync(path.join(site, "out"), path.join(out, v.id), { recursive: true });
    console.log(`build-versions: ${v.id} built from ${v.tag} into out/${v.id}/`);
  } finally {
    run(`git worktree remove --force "${wt}"`, repo);
  }
}
console.log("build-versions: done");
