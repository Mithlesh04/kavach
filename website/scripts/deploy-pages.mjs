// Publishes the site to GitHub Pages: builds it to serve under
// /<repository>/ (https://<owner>.github.io/<repository>/), then commits
// out/ to the gh-pages branch and pushes it. GitHub serves that branch
// once Settings → Pages → "Deploy from a branch" names gh-pages, / (root).
//
// Usage: npm run deploy:pages   (run `npm run check` first)
//
// The commit is made with a separate index in the git-ignored .kavach/
// folder, so the checkout and its staged changes are never touched.

import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const repo = path.resolve(root, "..");
const git = (args, env = {}) =>
  execFileSync("git", args, { cwd: repo, encoding: "utf8", env: { ...process.env, ...env } }).trim();

const remote = git(["remote", "get-url", "origin"]);
const found = remote.match(/github\.com[:/]([^/]+)\/([^/.]+)/);
if (!found) {
  console.error(`deploy-pages: origin is not a GitHub repository: ${remote}`);
  process.exit(1);
}
const [, owner, name] = found;
const base = `/${name}`;
const url = `https://${owner.toLowerCase()}.github.io/${name}`;

console.log(`deploy-pages: building for ${url}/`);
execSync("npm run build", {
  cwd: root,
  stdio: "inherit",
  env: { ...process.env, SITE_BASE_PATH: base, SITE_URL: url },
});

const out = path.join(root, "out");
// Without it, GitHub's Jekyll step drops _next/, which holds every script and style.
if (!fs.existsSync(path.join(out, ".nojekyll"))) throw new Error("deploy-pages: out/.nojekyll is missing (public/.nojekyll)");

const index = path.join(repo, ".kavach", "gh-pages.index");
fs.mkdirSync(path.dirname(index), { recursive: true });
fs.rmSync(index, { force: true });
const env = { GIT_INDEX_FILE: index };
// The built files go up exactly as written, without line-ending changes or warnings about them.
git(["-c", "core.autocrlf=false", "-c", "core.safecrlf=false", "--work-tree", out, "add", "--all", "--force", "."], env);
const tree = git(["write-tree"], env);
fs.rmSync(index, { force: true });

// Keep the branch's history: the new commit follows the last deploy.
let parent = [];
try {
  git(["fetch", "--quiet", "origin", "gh-pages"]);
  parent = ["-p", git(["rev-parse", "FETCH_HEAD"])];
} catch {
  console.log("deploy-pages: no gh-pages branch yet; this is its first commit");
}
const head = git(["rev-parse", "--short", "HEAD"]);
const commit = git(["commit-tree", tree, ...parent, "-m", `Deploy the site from ${head}`]);
git(["push", "origin", `${commit}:refs/heads/gh-pages`]);
console.log(`deploy-pages: pushed ${commit.slice(0, 7)} to gh-pages; the site is at ${url}/`);
console.log("deploy-pages: out/ is now built for GitHub Pages; `npm run build` builds it for the root again");
