import fs from "node:fs";
import path from "node:path";
import outlines from "@/data/outlines.json";

// The public declarations of the standard library, read from
// data/outlines.json (scripts/build-outlines.mjs, which asks
// `kavach outline` for each file): what the Reference shows of each
// module. A declaration keeps its doc comment, its signature with its
// contracts and `example`s, and the line where it starts in its file.

export type Decl = {
  kind: "type" | "interface" | "fn";
  name: string;
  /** The `///` comment above it, as one paragraph. */
  doc: string;
  /** What a reader writes against: a type's fields and invariants, a function's signature, contracts and examples. */
  code: string;
  file: string;
  line: number;
  /** A type's public methods, its own and those of `extend` blocks in the same module. */
  methods: Decl[];
};

const repo = path.join(process.cwd(), "..");
const all = outlines as Record<string, string>;

/** Each stdlib file's module name, as `use` writes it, from the compiler's own list (compiler/driver/src/std_modules.rs). */
const moduleNames: Map<string, string> = (() => {
  const src = fs.readFileSync(path.join(repo, "compiler", "driver", "src", "std_modules.rs"), "utf8");
  const pairs = [...src.matchAll(/"(std\.[\w.]+)",\s*include_str!\("(?:\.\.\/)+(stdlib\/[\w/]+\.kav)"\)/g)];
  return new Map(pairs.map(([, name, file]) => [file, name]));
})();

/** The module a stdlib file is: `stdlib/http/hpack.kav` is `std.http.hpack`. */
export function moduleNameOf(file: string): string {
  const name = moduleNames.get(file);
  if (!name) throw new Error(`outline: compiler/driver/src/std_modules.rs names no module for ${file}`);
  return name;
}

/** Every folder of stdlib/ with a .kav file: `fs`, `http`, … */
export const stdlibFolders = (): string[] => [...new Set(Object.keys(all).map((f) => f.split("/")[1]))];

/** The stdlib files of a module folder (`http` gives `stdlib/http/*.kav`), in order. */
export const filesOf = (folder: string): string[] => Object.keys(all).filter((f) => f.startsWith(`stdlib/${folder}/`));

/** A declaration as shown: bodies are left out, so the `{ … }` that stands for one goes too. */
const shown = (lines: string[]) =>
  lines
    .filter((l) => l.trim() !== "{ … }")
    .map((l) => l.replace(/ \{ … \}$/, ""))
    .join("\n");

/** `{`s minus `}`s outside strings and `{ … }` bodies. */
function depthChange(line: string): number {
  const bare = line.replace(/"(?:\\.|[^"\\])*"/g, '""').replace(/\{ … \}/g, "");
  return (bare.match(/\{/g) ?? []).length - (bare.match(/\}/g) ?? []).length;
}

/** Splits lines into blocks at blank lines that sit at depth `at`. */
function blocks(lines: string[], at = 0): string[][] {
  const out: string[][] = [];
  let current: string[] = [];
  let depth = at;
  for (const line of lines) {
    if (!line.trim() && depth === at) {
      if (current.length) out.push(current);
      current = [];
      continue;
    }
    current.push(line);
    depth += depthChange(line);
  }
  if (current.length) out.push(current);
  return out;
}

const dedent = (lines: string[]) => {
  const n = Math.min(...lines.filter((l) => l.trim()).map((l) => l.match(/^ */)![0].length));
  return lines.map((l) => l.slice(n));
};

/** The doc comment and the rest of a block. */
function split(block: string[]): { doc: string; head: string[] } {
  const i = block.findIndex((l) => !l.trim().startsWith("///"));
  const doc = block
    .slice(0, i)
    .map((l) => l.trim().replace(/^\/\/\/ ?/, ""))
    .join(" ");
  return { doc, head: block.slice(i) };
}

/** The line, from 1, where `pattern` first matches in `file` at or after line `from`. */
function lineOf(file: string, pattern: RegExp, from = 1): number {
  const lines = fs.readFileSync(path.join(repo, file), "utf8").split(/\r?\n/);
  const i = lines.findIndex((l, n) => n + 1 >= from && pattern.test(l));
  if (i < 0) throw new Error(`outline: ${file} has no line matching ${pattern} (rebuild data/outlines.json)`);
  return i + 1;
}

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** A type's or extend block's public methods. */
function methodsIn(inner: string[], file: string, from: number): Decl[] {
  return blocks(dedent(inner))
    .map(split)
    .filter(({ head }) => /^pub fn \w+/.test(head[0]))
    .map(({ doc, head }) => {
      const name = head[0].match(/^pub fn (\w+)/)![1];
      return { kind: "fn", name, doc, code: shown(head), file, line: lineOf(file, new RegExp(`^\\s*pub fn ${esc(name)}\\b`), from), methods: [] };
    });
}

type Extend = { name: string; methods: Decl[] };

/** Every public type, interface and function a file declares, and the methods its `extend` blocks add. */
function declsOf(file: string): { found: Decl[]; extended: Extend[] } {
  const text = all[file];
  if (text === undefined) throw new Error(`outline: no ${file} in data/outlines.json (run scripts/build-outlines.mjs)`);
  const found: Decl[] = [];
  const extended: Extend[] = [];
  for (const { doc, head } of blocks(text.split("\n")).map(split)) {
    const first = head[0] ?? "";
    const opens = first.trimEnd().endsWith("{");
    const inner = opens ? head.slice(1, -1) : [];
    const typed = first.match(/^pub (type|interface) (\w+)/);
    if (typed) {
      const [, kind, name] = typed;
      const line = lineOf(file, new RegExp(`^pub ${kind} ${esc(name)}\\b`));
      // A type shows its fields and invariants; its methods are listed apart.
      const own = blocks(dedent(inner)).filter((b) => !/^pub fn |^fn /.test(split(b).head[0]));
      const body = own.map((b) => b.map((l) => `    ${l}`).join("\n")).join("\n\n");
      const code = kind === "interface" ? shown(head) : opens ? `${first}\n${body}\n}` : shown(head);
      const methods = kind === "type" ? methodsIn(inner, file, line) : [];
      found.push({ kind: kind as "type" | "interface", name, doc, code, file, line, methods });
      continue;
    }
    const ext = first.match(/^extend(?:<[^>]*>)? (\w+)/);
    if (ext && opens) {
      const line = lineOf(file, new RegExp(`^${esc(first.replace(/\s*\{$/, ""))}`));
      extended.push({ name: ext[1], methods: methodsIn(inner, file, line) });
      continue;
    }
    const fn = first.match(/^pub fn (\w+)/);
    if (fn) found.push({ kind: "fn", name: fn[1], doc, code: shown(head), file, line: lineOf(file, new RegExp(`^pub fn ${esc(fn[1])}\\b`)), methods: [] });
  }
  return { found, extended };
}

/**
 * Every public declaration of a module's files. Methods an `extend` block
 * adds join their type when the module declares it; others, such as
 * methods added to `String`, are listed under the type's name alone.
 */
export function moduleDecls(files: string[]): Decl[] {
  const each = files.map(declsOf);
  const found = each.flatMap((e) => e.found);
  for (const { name, methods } of each.flatMap((e) => e.extended)) {
    if (!methods.length) continue;
    const owner = found.find((d) => d.kind === "type" && d.name === name);
    if (owner) owner.methods.push(...methods);
    else found.push({ kind: "type", name, doc: "", code: "", file: methods[0].file, line: methods[0].line, methods });
  }
  return found;
}
