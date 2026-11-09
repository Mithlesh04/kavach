// The sample guard (decision 0033): every sample in content/samples/ is
// run with the real kavach, and what it gives must match its .expect
// file exactly. A docs sample that lies fails the check.
//
// A sample is a file `name.kav` or a project folder `name.project/`
// (kavach.toml + src/), and its name says what is checked — see
// lib/sample-kinds.mjs. Each one runs in a fresh folder under the
// repository's git-ignored .kavach/, so samples that write files (or
// `kavach test`'s tests/found/) never touch the repository.
//
// Without a kavach binary (e.g. on a plain web host) it warns and skips,
// so the guard runs where the compiler lives: dev machines and CI.

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { findKavach } from "../lib/find-kavach.mjs";
import { inputOf, kindOf, shownName } from "../lib/sample-kinds.mjs";

const root = process.cwd();
const samplesDir = path.join(root, "content", "samples");
const runsDir = path.join(root, "..", ".kavach", "sample-runs");

/** Sample files and project folders (a project's own files are not samples). */
function* samples(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name.endsWith(".project")) yield p;
    else if (entry.isDirectory()) yield* samples(p);
    else if (entry.name.endsWith(".kav")) yield p;
  }
}

/** The kavach arguments for a sample of this kind; no file in project mode. */
function argsFor(kind, file) {
  const target = file ? [file] : [];
  switch (kind) {
    case "dry":
      return ["run", "--dry-run", ...target];
    case "native":
      return ["run", "--native", ...target];
    case "test":
      return ["test", ...target, "--seed", "7"];
    case "lib":
      return ["build", "--lib", ...target, "-o", "out"];
    default:
      return ["run", ...target];
  }
}

/**
 * A story across runs: a project with a `runs` file, one `kavach` command
 * per line (`run src/main.kav power-cut`), run in order in the same
 * folder. A line may start with settings (`KAVACH_GPU=on run …`). The
 * result reads like the terminal: each command, then what it printed,
 * its standard output and then its standard error (where build, check
 * and fmt report). A run may stop with an error — that can be the story.
 */
function story(kavach, dir) {
  const lines = fs.readFileSync(path.join(dir, "runs"), "utf8").split(/\r?\n/).filter((l) => l.trim());
  let out = "";
  for (const line of lines) {
    const words = line.trim().split(/\s+/);
    const env = { ...process.env };
    const set = [];
    while (/^[A-Z_]+=/.test(words[0])) {
      const [name, value] = words[0].split("=");
      env[name] = value;
      set.push(words.shift());
    }
    const r = spawnSync(kavach, words, { cwd: dir, env, encoding: "utf8", timeout: 120_000 });
    if (r.error) throw r.error;
    const printed = [r.stdout.trimEnd(), r.stderr.trimEnd()].filter(Boolean).join("\n");
    out += `$ ${[...set, "kavach", ...words].join(" ")}\n${printed}\n`;
  }
  return out;
}

/**
 * A .warns sample: it must run and print at least one warning (warnings
 * go to standard error, which a plain run doesn't record). Gives the
 * `warning:` lines, then what the program printed, then its `panic:`
 * line when the warning was right and the run ends in one.
 */
function warned(kavach, file, dir) {
  const r = spawnSync(kavach, argsFor("run", file), { cwd: dir, encoding: "utf8", timeout: 120_000 });
  if (r.error) throw r.error;
  const lines = r.stderr.split(/\r?\n/);
  const panic = lines.find((l) => l.startsWith("panic:"));
  if (r.status !== 0 && !panic) throw new Error(`fails to run\n${r.stderr}`);
  const warnings = lines.filter((l) => l.startsWith("warning:"));
  if (!warnings.length) throw new Error("a .warns sample must print a warning, but printed none");
  return [warnings.join("\n"), r.stdout.trimEnd(), panic].filter(Boolean).join("\n");
}

/**
 * A .dry sample: what the program printed, then each `[dry run]` line
 * (a simulated capability call, written to standard error). The two
 * streams can't be interleaved faithfully, so the calls always come last.
 */
function dryRun(kavach, file, dir) {
  const r = spawnSync(kavach, argsFor("dry", file), { cwd: dir, encoding: "utf8", timeout: 120_000 });
  if (r.error) throw r.error;
  if (r.status !== 0) throw new Error(`fails to run\n${r.stderr}`);
  const calls = r.stderr.split(/\r?\n/).filter((l) => l.startsWith("[dry run]"));
  return [r.stdout.trimEnd(), ...calls].filter(Boolean).join("\n");
}

/** Runs one sample in `dir`; gives what it printed, or throws a message. */
function run(kavach, kind, file, dir, input) {
  if (!file && fs.existsSync(path.join(dir, "runs"))) return story(kavach, dir);
  if (kind === "warns") return warned(kavach, file, dir);
  if (kind === "dry") return dryRun(kavach, file, dir);
  // What a person would type, when the sample has a .stdin file.
  const stdio = [input === undefined ? "ignore" : "pipe", "pipe", "pipe"];
  const opts = { cwd: dir, encoding: "utf8", timeout: 120_000, stdio, input };
  const mustFail = kind === "fails" || kind === "panics";
  let out;
  try {
    out = execFileSync(kavach, argsFor(kind, file), opts);
    if (mustFail) throw new Error(`a .${kind} sample must fail, but it ran cleanly`);
  } catch (e) {
    if (e.status === undefined) throw e;
    const errLines = String(e.stderr ?? "").split(/\r?\n/);
    if (kind === "panics") {
      const panic = errLines.find((l) => l.startsWith("panic:")) ?? "(no panic line)";
      out = `${String(e.stdout ?? "").trimEnd()}\n${panic}`;
    } else if (kind === "fails") {
      // The error, not a warning printed before it.
      out = errLines.find((l) => l.startsWith("error")) ?? errLines[0];
    } else if (kind === "test") {
      // A failing test is the point of some samples, but one that doesn't
      // compile tests nothing.
      const error = errLines.find((l) => l.startsWith("error"));
      if (error && !/\d+ passed, \d+ failed/.test(String(e.stdout ?? ""))) throw new Error(`doesn't compile\n${error}`);
      out = String(e.stdout ?? "");
    } else {
      throw new Error(`fails to run\n${e.stderr || e.message}`);
    }
  }
  if (kind === "lib") {
    const header = fs.readdirSync(path.join(dir, "out")).find((f) => f.endsWith(".h"));
    out = header ? fs.readFileSync(path.join(dir, "out", header), "utf8") : "(no header)";
  }
  // Paths print with \ on Windows; the page shows them as every other system does.
  return kind === "test" ? out.replace(/\\/g, "/") : out;
}

const kavach = findKavach(root);
if (!kavach) {
  console.warn("check-samples: no kavach binary found (set KAVACH_BIN) — samples NOT verified");
  process.exit(0);
}

let failed = 0;
let checked = 0;
fs.rmSync(runsDir, { recursive: true, force: true });
for (const sample of fs.existsSync(samplesDir) ? samples(samplesDir) : []) {
  checked++;
  const rel = path.relative(root, sample);
  const kind = kindOf(path.basename(sample));
  const isProject = sample.endsWith(".project");
  const dir = path.join(runsDir, String(checked));
  let out;
  try {
    if (isProject) {
      fs.cpSync(sample, dir, { recursive: true });
      out = run(kavach, kind, null, dir);
    } else {
      fs.mkdirSync(dir, { recursive: true });
      const shown = shownName(path.basename(sample));
      fs.copyFileSync(sample, path.join(dir, shown));
      const input = inputOf(sample, fs.existsSync, (p) => fs.readFileSync(p, "utf8"));
      out = run(kavach, kind, shown, dir, input);
    }
  } catch (e) {
    console.error(`FAILS         ${rel}: ${e.message}`);
    failed++;
    continue;
  }
  // A plain sample that warns would hide the warning from the page, so it
  // must be a .warns sample. Operator-table samples (t-) print one result
  // per row and are read only for those results.
  if (kind === "run" && !path.basename(sample).startsWith("t-") && !fs.existsSync(path.join(dir, "runs"))) {
    const c = spawnSync(kavach, isProject ? ["check"] : ["check", shownName(path.basename(sample))], { cwd: dir, encoding: "utf8", timeout: 120_000 });
    const warning = `${c.stdout}${c.stderr}`.split(/\r?\n/).find((l) => l.startsWith("warning:"));
    if (warning) {
      console.error(`HIDES WARNING ${rel}: name it .warns so the page shows it\n  ${warning}`);
      failed++;
      continue;
    }
  }
  const expectPath = sample.replace(/\.(kav|project)$/, ".expect");
  if (!fs.existsSync(expectPath) && process.argv.includes("--write")) {
    fs.writeFileSync(expectPath, `${out.replace(/\r\n/g, "\n").trim()}\n`);
    console.log(`WROTE         ${path.relative(root, expectPath)} — read it before you commit it`);
    continue;
  }
  if (!fs.existsSync(expectPath)) {
    console.error(`NO EXPECT     ${rel}: run \`node scripts/check-samples.mjs --write\` and read what it wrote`);
    failed++;
    continue;
  }
  const got = out.replace(/\r\n/g, "\n").trim();
  const want = fs.readFileSync(expectPath, "utf8").replace(/\r\n/g, "\n").trim();
  if (got !== want) {
    console.error(`WRONG OUTPUT  ${rel}\n  expected: ${JSON.stringify(want)}\n  got:      ${JSON.stringify(got)}`);
    failed++;
  }
}
fs.rmSync(runsDir, { recursive: true, force: true });

// The mistakes benchmark's table is shown on the site as it is, so each
// row's step and message must still be what `kavach check` gives first.
const mistakes = path.join(root, "..", "benchmarks", "mistakes");
const rows = fs
  .readFileSync(path.join(mistakes, "README.md"), "utf8")
  .split(/\r?\n/)
  .map((l) => l.match(/^\| \[.*\]\(cases\/([^)]+)\) \| ([^|]+) \| (.*) \|$/))
  .filter(Boolean);
for (const [, file, step, message] of rows) {
  checked++;
  const r = spawnSync(kavach, ["check", path.join(mistakes, "cases", file)], { encoding: "utf8", timeout: 120_000 });
  const first = `${r.stdout}${r.stderr}`.split(/\r?\n/).map((l) => l.match(/^(error|warning): (.*)/)).find(Boolean);
  const kind = step.startsWith("compile error") ? "error" : step.trim() === "only when it runs" ? null : "warning";
  const want = message.replace(/\\\|/g, "|");
  if ((first?.[1] ?? null) !== kind || (first && first[2] !== want)) {
    console.error(`WRONG ROW     benchmarks/mistakes/README.md ${file}\n  expected: ${kind} ${want}\n  got:      ${first ? `${first[1]} ${first[2]}` : "nothing"}`);
    failed++;
  }
}

if (failed) {
  console.error(`check-samples: ${failed} of ${checked} samples lie — fix them before shipping docs`);
  process.exit(1);
}
console.log(`check-samples: ${checked} samples verified against ${path.relative(root, kavach)}`);
