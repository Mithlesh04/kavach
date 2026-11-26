// Checks that every page showing an explainer scene fits screens from a
// phone's 320 px to a 2560 px monitor: the page never scrolls sideways,
// and nothing in a scene sticks out of the screen. Wide parts, such as code
// lines and a timeline, may scroll sideways inside their own box.
//
// Usage: node scripts/check-widths.mjs   (after `npm run build`)
// It needs Chrome or Edge; CHROME names another browser's program. It
// serves out/ as GitHub Pages does and drives the browser through its
// DevTools protocol, with a fresh profile in the git-ignored .kavach/.

import { spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";

const root = process.cwd();
const out = path.join(root, "out");
const WIDTHS = [320, 2560];
const PORT = 8099;
const DEBUG = 9333;

const browsers = [
  process.env.CHROME,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
].filter(Boolean);
const browser = browsers.find((b) => fs.existsSync(b));
if (!fs.existsSync(out)) throw new Error("check-widths: run `npm run build` first — out/ is missing");
if (!browser) throw new Error("check-widths: no Chrome or Edge found; set CHROME to one");

/** Every built page with a scene on it, as a path. */
const pages = fs
  .readdirSync(out, { recursive: true })
  .filter((f) => f.endsWith("index.html") && fs.readFileSync(path.join(out, f), "utf8").includes('id="scene-'))
  .map((f) => `/${path.dirname(f).split(path.sep).join("/")}/`.replace("//", "/"))
  .sort();

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
const server = http.createServer((req, res) => {
  let file = path.join(out, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
  if (!fs.existsSync(file)) return res.writeHead(404).end();
  res.writeHead(200, { "content-type": types[path.extname(file)] ?? "application/octet-stream" });
  res.end(fs.readFileSync(file));
});
await new Promise((r) => server.listen(PORT, "127.0.0.1", r));

const profile = path.join(root, "..", ".kavach", "check-widths", String(Date.now()));
fs.mkdirSync(profile, { recursive: true });
const chrome = spawn(browser, ["--headless=new", `--remote-debugging-port=${DEBUG}`, `--user-data-dir=${profile}`, "--no-first-run", "about:blank"]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let ws;
for (let i = 0; i < 40 && !ws; i++) {
  await sleep(250);
  try {
    const tab = (await (await fetch(`http://127.0.0.1:${DEBUG}/json/list`)).json()).find((t) => t.type === "page");
    if (tab) ws = new WebSocket(tab.webSocketDebuggerUrl);
  } catch {
    // The browser is still starting.
  }
}
if (!ws) throw new Error("check-widths: the browser didn't start");
await new Promise((r) => ws.addEventListener("open", r));
let id = 0;
const waiting = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  waiting.get(m.id)?.(m);
  waiting.delete(m.id);
});
const send = (method, params = {}) =>
  new Promise((r) => {
    waiting.set(++id, r);
    ws.send(JSON.stringify({ id, method, params }));
  });

// In the page: how far it scrolls sideways, and each scene's parts that stick
// out of the screen, not counting those inside a box that scrolls sideways.
const measure = `JSON.stringify((() => {
  const vw = document.documentElement.clientWidth;
  const out = [];
  for (const scene of document.querySelectorAll('[id^=scene-]')) {
    for (const el of scene.querySelectorAll('*')) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || (r.right <= vw + 1 && r.left >= -1)) continue;
      let a = el.parentElement, scrolls = false;
      while (a && a !== scene.parentElement) {
        const ox = getComputedStyle(a).overflowX;
        if (ox === 'auto' || ox === 'scroll') { scrolls = true; break; }
        a = a.parentElement;
      }
      if (!scrolls) out.push(scene.id + ': ' + el.tagName.toLowerCase() + ' "' + (el.textContent || '').trim().slice(0, 30) + '"');
    }
  }
  return { sideways: document.documentElement.scrollWidth - vw, out };
})())`;

let failed = 0;
for (const width of WIDTHS) {
  await send("Emulation.setDeviceMetricsOverride", { width, height: 900, deviceScaleFactor: 1, mobile: width < 600 });
  for (const page of pages) {
    await send("Page.navigate", { url: `http://127.0.0.1:${PORT}${page}` });
    await sleep(1500);
    const m = JSON.parse((await send("Runtime.evaluate", { expression: measure, returnByValue: true })).result.result.value);
    if (m.sideways > 0 || m.out.length > 0) {
      failed++;
      console.error(`check-widths: ${page} at ${width} px: ${m.sideways > 0 ? `the page scrolls ${m.sideways} px sideways; ` : ""}${m.out.slice(0, 5).join("; ")}`);
    }
  }
}
ws.close();
chrome.kill();
server.close();
if (failed > 0) {
  console.error(`check-widths: ${failed} of ${pages.length * WIDTHS.length} page widths don't fit`);
  process.exit(1);
}
console.log(`check-widths: ${pages.length} pages with scenes fit at ${WIDTHS.join(" and ")} px`);
