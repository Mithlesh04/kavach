import fs from "node:fs";
import path from "node:path";

// Which course page shows each sample, worked out from the lesson files
// at build time, so a list of samples (the compiler's messages in the
// Reference) can link each one to the page that explains it. Lessons
// can't be imported here (the Reference is a lesson), so their source is
// read as text: content/lessons/index.ts maps course keys to lesson
// constants, and each lesson file's top-level declarations hold sample
// names as string literals, directly or through other declarations.

const root = process.cwd();
const lessonsDir = path.join(root, "content", "lessons");
const samplesDir = path.join(root, "content", "samples");

/** Every sample's name, as pages use it: `habits/null.fails`, `literals/sql`. */
export function sampleNames(): string[] {
  const out: string[] = [];
  const walk = (dir: string, rel: string) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const r = rel ? `${rel}/${e.name}` : e.name;
      if (e.isDirectory() && e.name.endsWith(".project")) out.push(r.replace(/\.project$/, ""));
      else if (e.isDirectory()) walk(path.join(dir, e.name), r);
      else if (e.name.endsWith(".kav")) out.push(r.replace(/\.kav$/, ""));
    }
  };
  walk(samplesDir, "");
  return out;
}

/** A file's top-level declarations: name → its text. */
function declarations(text: string): Map<string, string> {
  const out = new Map<string, string>();
  const starts = [...text.matchAll(/^(?:export )?(?:const|function|async function) (\w+)/gm)];
  starts.forEach((m, i) => out.set(m[1], text.slice(m.index, starts[i + 1]?.index ?? text.length)));
  return out;
}

/** Sample name → every course key whose page shows it. */
export function samplePages(): Map<string, string[]> {
  const known = new Set(sampleNames());
  const index = fs.readFileSync(path.join(lessonsDir, "index.ts"), "utf8");
  const fileOf = new Map<string, string>();
  for (const [, names, file] of index.matchAll(/import \{([^}]+)\} from "\.\/([\w-]+)"/g)) {
    for (const n of names.split(",").map((s) => s.trim()).filter(Boolean)) fileOf.set(n, file);
  }
  const decls = new Map<string, Map<string, string>>();
  const declsOf = (file: string) => {
    if (!decls.has(file)) decls.set(file, declarations(fs.readFileSync(path.join(lessonsDir, `${file}.tsx`), "utf8")));
    return decls.get(file)!;
  };
  /** The samples a declaration names, and those of the declarations it uses, in the same file. */
  const samplesIn = (file: string, name: string, seen = new Set<string>()): string[] => {
    if (seen.has(name)) return [];
    seen.add(name);
    const all = declsOf(file);
    const text = all.get(name) ?? "";
    const own = [...text.matchAll(/"([\w./-]+)"/g)].map((m) => m[1]).filter((s) => known.has(s));
    const used = [...all.keys()].filter((k) => k !== name && new RegExp(`\\b${k}\\b`).test(text));
    return [...own, ...used.flatMap((k) => samplesIn(file, k, seen))];
  };
  const pages = new Map<string, string[]>();
  for (const [, key, doc] of index.matchAll(/^\s*"([\w/-]+)": (\w+),$/gm)) {
    const file = fileOf.get(doc);
    if (!file) continue;
    for (const s of samplesIn(file, doc)) pages.set(s, [...(pages.get(s) ?? []), key]);
  }
  return pages;
}
