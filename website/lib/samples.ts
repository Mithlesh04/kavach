import fs from "node:fs";
import path from "node:path";
import { shownName } from "./sample-kinds.mjs";

/**
 * Code samples and their expected output, read from content/samples/.
 * The same files are what scripts/check-samples.mjs runs with the real
 * kavach at check time — so what the page shows is, by construction,
 * what the language does. Name endings: lib/sample-kinds.mjs.
 */

const samplesDir = path.join(process.cwd(), "content", "samples");

function readExpect(base: string): string | undefined {
  try {
    return fs.readFileSync(`${base}.expect`, "utf8").replace(/\r\n/g, "\n").trim();
  } catch {
    return undefined;
  }
}

/** A one-file sample: its code, what it prints, and the file name to show. */
export function loadSample(name: string): { code: string; output?: string; file: string } {
  const base = path.join(samplesDir, name);
  // A refused sample (name.fails.kav) or a test sample needs an empty
  // main to be a whole program; the page doesn't show it.
  const code = fs
    .readFileSync(`${base}.kav`, "utf8")
    .replace(/\r\n/g, "\n")
    .replace(/\n+fn main\(\) \{\n?\}\s*$/, "")
    .trimEnd();
  return { code, output: readExpect(base), file: shownName(path.basename(`${base}.kav`)) };
}

/** The sample's body without `fn main() { … }`, for teaching snippets. */
export function sampleBody(name: string): { code: string; output?: string; file: string } {
  const sample = loadSample(name);
  const m = sample.code.match(/^fn main\(\) \{\n([\s\S]*)\n\}$/);
  if (!m) return sample;
  const body = m[1]
    .split("\n")
    .map((l) => (l.startsWith("    ") ? l.slice(4) : l))
    .join("\n");
  return { ...sample, code: body };
}

/** Every `.kav` file under a project's src/, by path inside the project. */
function kavFiles(base: string): string[] {
  const src = path.join(base, "src");
  if (!fs.existsSync(src)) return [];
  return fs
    .readdirSync(src, { recursive: true, encoding: "utf8" })
    .filter((f) => f.endsWith(".kav"))
    .map((f) => `src/${f.replace(/\\/g, "/")}`)
    .sort();
}

/**
 * All of a project's program, ready to show as tabs: `first`, then
 * src/main.kav, then the rest in order. No file can be left out by mistake.
 */
export function loadProjectFiles(name: string, first: string[] = []): { files: { path: string; code: string }[]; output?: string } {
  const all = kavFiles(path.join(samplesDir, `${name}.project`));
  return loadProject(name, [...new Set([...first, "src/main.kav", ...all])].filter((f) => all.includes(f)));
}

/**
 * A project sample (`name.project/`: kavach.toml + src/): the files to
 * show, by path inside the project, and what running it prints.
 */
export function loadProject(name: string, files: string[]): { files: { path: string; code: string }[]; output?: string } {
  const base = path.join(samplesDir, `${name}.project`);
  // A page that shows the project's program shows all of it: code a reader can't open can't teach them.
  if (files.some((f) => f.startsWith("src/"))) {
    const hidden = kavFiles(base).filter((f) => !files.includes(f));
    if (hidden.length) throw new Error(`${name}: the page doesn't show ${hidden.join(", ")}: use <Files> for a project of several files`);
  }
  return {
    files: files.map((f) => ({
      path: f,
      code: fs.readFileSync(path.join(base, f), "utf8").replace(/\r\n/g, "\n").trimEnd(),
    })),
    output: readExpect(path.join(samplesDir, name)),
  };
}
