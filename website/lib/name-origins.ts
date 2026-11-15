import { course, slugOf } from "@/lib/course";
import { fileFigure } from "@/lib/spec-tables";
import { filesOf, moduleDecls, moduleNameOf, stdlibFolders } from "@/lib/outline";

// Where each name in a sample comes from. A reader meets `BadQty`,
// `AgentSettings` or `Console` in a sample and should not have to guess:
// declared in this program (and on which line), a variant of one of its
// unions, a type of a standard module (which every program has without
// `use`, spec 8.4 and 14.7), built in, or a built-in capability. All of
// it is worked out from the sample's own code and the compiler's lists
// (compiler/resolver/src/prelude.rs, data/outlines.json), never typed.

export type Origin = {
  name: string;
  kind: "own" | "variant" | "std" | "builtin" | "capability";
  /** For "own" and "variant": where it is declared. */
  at?: { file?: string; line: number };
  /** For "variant": its union. */
  union?: string;
  /** For "std": its module, and whether the sample has a `use` of it. */
  module?: string;
  used?: boolean;
  href?: string;
};

/** A Rust list of names in prelude.rs: `pub const CAPABILITIES: &[&str] = &[ … ];`. */
function preludeList(pattern: RegExp): Set<string> {
  return new Set([...fileFigure("compiler/resolver/src/prelude.rs", pattern).matchAll(/"(\w+)"/g)].map((m) => m[1]));
}
const BUILT_IN_TYPES = preludeList(/pub\(crate\) const TYPES: &\[&str\] = &\[([\s\S]*?)\];/);
const CAPABILITIES = preludeList(/pub const CAPABILITIES: &\[&str\] = &\[([\s\S]*?)\];/);

/** Written in Kavach, but taught as built in (Reference › Built-in Types). */
const TAUGHT_AS_BUILT_IN = new Set(["Json", "JsonError", "Taken"]);

const stdPages = course.find((p) => p.slug === "std")!.pages.slice(1);

/** The page of a stdlib folder: the Standard Library Reference page whose title names `std.<folder>`. */
function pageOf(folder: string): string {
  const page = stdPages.find((s) => [...s.title.matchAll(/std\.(\w+)/g)].some((m) => m[1] === folder));
  if (!page) throw new Error(`name-origins: no Standard Library Reference page names std.${folder} (lib/course.ts)`);
  return `/docs/std/${slugOf(page.title)}/`;
}

/** The address reference-std.tsx gives a declaration's heading. */
const anchor = (mod: string, name: string) => `${mod}-${name}`.toLowerCase().replace(/[^a-z0-9]+/g, "-");

/** Every standard type by name; a name two modules declare is left out, since a note couldn't say which. */
const stdTypes: Map<string, { mod: string; folder: string; href: string }> = (() => {
  const seen = new Map<string, { mod: string; folder: string; href: string }[]>();
  for (const folder of stdlibFolders()) {
    for (const d of moduleDecls(filesOf(folder))) {
      if (d.kind === "fn" || !d.code) continue;
      const mod = moduleNameOf(d.file);
      seen.set(d.name, [...(seen.get(d.name) ?? []), { mod, folder, href: `${pageOf(folder)}#${anchor(mod, d.name)}` }]);
    }
  }
  return new Map([...seen].filter(([, all]) => all.length === 1).map(([name, [one]]) => [name, one]));
})();

/** A line without its comment and the text of its strings (what is inside `{…}` stays). */
function codeOf(line: string): string {
  let out = "";
  let inString = false;
  let depth = 0;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (!inString && c === "/" && line[i + 1] === "/") break;
    if (inString && depth === 0) {
      if (c === "\\") i++;
      else if (c === '"') inString = false;
      else if (c === "{") depth = 1;
      out += " ";
      continue;
    }
    if (inString && c === "{") depth++;
    if (inString && c === "}" && --depth === 0) {
      out += " ";
      continue;
    }
    if (!inString && c === '"') {
      inString = true;
      out += " ";
      continue;
    }
    out += c;
  }
  return out;
}

/** The first name of each `|`-separated part, outside brackets: the variants of `A(x: Int) | B`. */
function variantsOf(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let part = "";
  for (const c of text) {
    if ("([{<".includes(c)) depth++;
    if (")]}>".includes(c)) depth--;
    if (c === "|" && depth === 0) {
      parts.push(part);
      part = "";
    } else part += c;
  }
  parts.push(part);
  return parts.map((p) => p.trim().match(/^([A-Z]\w*)/)?.[1]).filter((n): n is string => !!n);
}

type Declared = { name: string; file?: string; line: number; union?: string };

/** What a file declares: types, interfaces, agents, capabilities, and the variants of its unions. */
function declarations(code: string, file?: string): { decls: Declared[]; generics: Set<string> } {
  const lines = code.split("\n").map(codeOf);
  const decls: Declared[] = [];
  const generics = new Set<string>();
  lines.forEach((l, i) => {
    for (const g of l.matchAll(/<([^<>()]*)>/g)) {
      for (const p of g[1].split(",")) {
        const n = p.trim().match(/^([A-Z]\w*)\s*(?::|$)/)?.[1];
        if (n && n.length <= 2) generics.add(n);
      }
    }
    const d = l.match(/^\s*(?:pub\s+)?(?:type|interface|agent|capability)\s+([A-Z]\w*)/);
    if (!d) return;
    decls.push({ name: d[1], file, line: i + 1 });
    const union = l.match(/^\s*(?:pub\s+)?type\s+[A-Z]\w*(?:<[^>]*>)?\s*=(.*)$/);
    if (!union) return;
    // A union may go on over the next lines, each starting with `|`.
    let text = union[1];
    const at = [i + 1];
    for (let j = i + 1; j < lines.length && /^\s*\|/.test(lines[j]); j++) {
      text += ` ${lines[j]}`;
      at.push(j + 1);
    }
    const names = variantsOf(text);
    // `type Score = Int` is another name for a type, not a union of one variant.
    const alias = names.length === 1 && !/[(|]/.test(text) && (BUILT_IN_TYPES.has(names[0]) || stdTypes.has(names[0]));
    if (alias) return;
    for (const v of names) {
      const line = at.find((n) => new RegExp(`\\b${v}\\b`).test(lines[n - 1])) ?? i + 1;
      decls.push({ name: v, file, line, union: d[1] });
    }
  });
  return { decls, generics };
}

/**
 * Every capitalised name a sample uses, with where it comes from, in the
 * order the code first uses it. `files[0]` is the file shown first; the
 * others are named by path.
 */
export function originsIn(files: { path?: string; code: string }[]): Origin[] {
  const declared = files.map((f, i) => declarations(f.code, i === 0 ? undefined : f.path));
  const decls = declared.flatMap((d) => d.decls);
  const generics = new Set(declared.flatMap((d) => [...d.generics]));
  const used = new Set(files.flatMap((f) => [...f.code.matchAll(/^\s*use\s+std\.(\w+)/gm)].map((m) => m[1])));
  const out = new Map<string, Origin>();
  const add = (o: Origin) => {
    if (!out.has(o.name)) out.set(o.name, o);
  };
  for (const f of files) {
    const code = f.code.split("\n").map(codeOf).join("\n");
    for (const m of code.matchAll(/(?:\b([A-Z]\w*)\.)?\b([A-Z]\w*)\b/g)) {
      const [, before, name] = m;
      if (before) {
        // `OrderError.BadQty`: the union first, then its variant.
        resolve(before);
        const variant = decls.find((d) => d.union === before && d.name === name);
        if (variant) add({ name: `${before}.${name}`, kind: "variant", union: before, at: { file: variant.file, line: variant.line } });
        continue;
      }
      resolve(name);
    }
  }
  function resolve(name: string) {
    if (generics.has(name) || name === "Self") return;
    const own = decls.find((d) => d.name === name && !d.union);
    if (own) return add({ name, kind: "own", at: { file: own.file, line: own.line } });
    if (CAPABILITIES.has(name)) return add({ name, kind: "capability", href: "/docs/agents/capabilities/" });
    if (BUILT_IN_TYPES.has(name) || TAUGHT_AS_BUILT_IN.has(name)) return add({ name, kind: "builtin", href: "/docs/reference/built-in-types/" });
    const std = stdTypes.get(name);
    if (std) return add({ name, kind: "std", module: std.mod, used: used.has(std.folder), href: std.href });
  }
  return [...out.values()];
}
