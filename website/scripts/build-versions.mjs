// The versioned multi-build (decision 0033): one static tree per docs
// version. `latest` is the ordinary build; every other entry in
// data/versions.json builds from its git tag into out/<id>/ with
// next.config's basePath set, so its pages and file:line links stay
// pinned to that tag forever. A version marked `"retired": true` gets
// only a stub that redirects to latest with a notice.
//
// An older tree is given today's data/versions.json and version picker
// before it is built, so its picker lists every version and links to
// them, and shows a banner naming the version. Its pages are marked
// noindex, so search engines send readers to the latest docs.
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

/** Every .html file under `dir`. */
const pagesIn = (dir) =>
  fs.readdirSync(dir, { recursive: true }).filter((f) => f.endsWith(".html")).map((f) => path.join(dir, f));

/**
 * Marks an older version's pages noindex, so search engines send readers
 * to the latest docs: in the HTML itself, as a crawler reads it before any
 * script runs. The banner naming the version comes from the version
 * picker, as React replaces what is added to the body. The sitemap and
 * robots.txt go, as the site's own are at the root.
 */
function markOlder(dir) {
  for (const page of pagesIn(dir)) {
    const html = fs.readFileSync(page, "utf8");
    fs.writeFileSync(page, html.replace(/<head([^>]*)>/, `<head$1><meta name="robots" content="noindex"/>`));
  }
  for (const file of ["sitemap.xml", "robots.txt"]) fs.rmSync(path.join(dir, file), { force: true });
}

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
    for (const file of ["data/versions.json", "components/version-picker.tsx"]) {
      fs.copyFileSync(path.join(root, file), path.join(site, file));
    }
    run("npm ci", site);
    // An older tag may pin a Next.js that later had a security advisory, and
    // Next refuses to build with one. It is built with the Next.js this
    // build uses, which main keeps patched; the tag itself stays as it is.
    const next = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).dependencies.next;
    run(`npm install --no-save next@${next}`, site);
    // next.config.ts reads SITE_BASE_PATH, so this tree serves at /<id>/.
    execSync("npm run build", {
      cwd: site,
      stdio: "pipe",
      env: { ...process.env, SITE_BASE_PATH: `/${v.id}`, NEXT_PUBLIC_DOCS_VERSION: v.id },
    });
    const dir = path.join(out, v.id);
    fs.rmSync(dir, { recursive: true, force: true });
    fs.cpSync(path.join(site, "out"), dir, { recursive: true });
    markOlder(dir);
    console.log(`build-versions: ${v.id} built from ${v.tag} into out/${v.id}/`);
  } finally {
    run(`git worktree remove --force "${wt}"`, repo);
  }
}
console.log("build-versions: done");
