import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { codeHtml } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { course } from "@/lib/course";
import { findNode } from "@/lib/nav";
import { filesOf, moduleDecls, moduleNameOf, stdlibFolders, type Decl } from "@/lib/outline";
import { fileTables, specTables, type SpecTable } from "@/lib/spec-tables";
import { H2, Note, P } from "./parts";
import { Learn, Tables } from "./reference";
import { Cell } from "./spec-table";

// ---------- Reference › Standard Library ----------
//
// One page per module: the spec's tables of what it gives, then every
// public type, method and function its files declare, as `kavach outline`
// reads them (lib/outline.ts), each with its doc comment, its contracts,
// its `example`s and a link to the line that declares it. Nothing here is
// typed by hand: a change to the spec or to stdlib/ changes these pages.

const SPEC_DIR = path.join(process.cwd(), "..", "docs", "spec");
const CHAPTER = "14-standard-library";

/** Modules the spec describes outside chapter 14: file and section. */
const ELSEWHERE: Record<string, [string, string]> = {
  "std.json": ["02-types/12-json.md", "2.12"],
  "std.tasks": ["09-concurrency.md", "9.4"],
  "std.ai": ["08-capabilities-and-agents/04-typed-ai-calls.md", "8.4"],
  "std.literals": ["10-modules-and-packages/15-tagged-literals.md", "10.15"],
};

type StdModule = { title: string; key: string; file: string; number: string; folders: string[] };

/** Chapter 14's files, each with its section number and title: `14.3`, `std.fs`. */
const chapterFiles = fs
  .readdirSync(path.join(SPEC_DIR, CHAPTER))
  .filter((f) => /^\d\d-/.test(f) && !/^0[12]-/.test(f))
  .sort()
  .map((f) => {
    const [, number, title] = fs.readFileSync(path.join(SPEC_DIR, CHAPTER, f), "utf8").match(/^# (14\.\d+) (.+)/)!;
    return { file: `${CHAPTER}/${f}`, number, title: title.trim() };
  });

const page = course.find((p) => p.slug === "reference")!.pages.find((p) => p.slug === "standard-library")!;

/** The modules, in the order lib/course.ts lists them as the Standard Library page's sub-pages. */
export const stdModules: StdModule[] = page.subs.map((s) => {
  const spec = chapterFiles.find((c) => c.title === s.title);
  const [file, number] = spec ? [spec.file, spec.number] : ELSEWHERE[s.title] ?? [];
  if (!file) throw new Error(`Reference: no spec section is titled ${s.title} (lib/course.ts)`);
  const folders = [...s.title.matchAll(/std\.(\w+)/g)].map((m) => m[1]).filter((f) => filesOf(f).length);
  return { title: s.title, key: `reference/standard-library/${s.slug}`, file, number, folders };
});

// Every module has its page: a spec section or a stdlib folder without one fails the build.
for (const c of chapterFiles) {
  if (!stdModules.some((m) => m.title === c.title)) throw new Error(`Reference: spec ${c.number} ${c.title} has no page in lib/course.ts`);
}
for (const f of stdlibFolders()) {
  if (!stdModules.some((m) => m.folders.includes(f))) throw new Error(`Reference: stdlib/${f}/ belongs to no module page in lib/course.ts`);
}

/** A spec section's tables; none when it has none. */
function tablesOf(file: string, number: string): SpecTable[] {
  try {
    return specTables(file, number);
  } catch (e) {
    if (String(e).includes("no tables")) return [];
    throw e;
  }
}

/**
 * A declaration's code, highlighted, without an editor window around it:
 * a module such as std.http declares well over a hundred things, and a
 * window each would make its page several times larger.
 */
async function Code({ code }: { code: string }) {
  const html = await codeHtml(code);
  return (
    <div
      className="code-body overflow-x-auto rounded-[10px] border border-code-line bg-code-bg px-4 py-3 font-mono text-[13.5px] leading-[1.7] max-sm:text-[12.5px] [&>pre]:m-0 [&>pre]:bg-transparent!"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

const anchor = (module: string, name: string) => `${module}-${name}`.toLowerCase().replace(/[^a-z0-9]+/g, "-");

/** One declaration; `under` is the type a method belongs to, for its heading's address. */
function Declared({ d, module, under, method = false }: { d: Decl; module: string; under?: string; method?: boolean }) {
  const label = method ? "method" : d.kind === "fn" ? "function" : d.kind;
  return (
    <div className={`flex flex-col gap-2 ${method ? "border-l-2 border-line pl-4" : ""}`}>
      <h3 id={anchor(under ? `${module}-${under}` : module, d.name)} className="m-0 mt-2 text-[18px] font-bold">
        <C>{d.name}</C> <span className="text-[13px] font-normal text-soft">{label}</span>
      </h3>
      {d.doc ? <P><Cell text={d.doc} /></P> : null}
      {d.code ? <Code code={d.code} /> : null}
      <div><SrcRef file={d.file} line={d.line} label={`${d.file}:${d.line}`} /></div>
      {d.methods.map((m) => (
        <Declared key={`${m.name}-${m.line}`} d={m} module={module} under={d.name} method />
      ))}
    </div>
  );
}

const count = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** What one module's files declare, file by file. */
function Declarations({ m }: { m: StdModule }) {
  const files = m.folders.flatMap(filesOf);
  if (!files.length) {
    return <Note>Everything in {m.title} is built into the compiler and runtime, so the spec&apos;s tables above are the whole of it.</Note>;
  }
  const decls = moduleDecls(files);
  return (
    <>
      {files.map((file) => {
        const mine = decls.filter((d) => d.file === file);
        if (!mine.length) return null;
        const name = moduleNameOf(file);
        return (
          <div key={file} className="flex flex-col gap-3">
            <H2>{`${name} · ${file.split("/").pop()}`}</H2>
            <P>
              <C>{`use ${name}`}</C>: {count(mine.filter((d) => d.kind !== "fn").length, "type")} and{" "}
              {count(mine.filter((d) => d.kind === "fn").length, "function")}. <SrcRef file={file} label={file} />
            </P>
            {mine.map((d) => (
              <Declared key={`${d.name}-${d.line}`} d={d} module={name} />
            ))}
          </div>
        );
      })}
    </>
  );
}

function moduleDoc(m: StdModule): LessonDoc {
  return {
    lead: `Every type, method and function of ${m.title}: what the spec lists, then each declaration in its source, with its examples and the line that declares it.`,
    minutes: 5,
    body: (
      <>
        <Learn number={m.number} />
        <P>
          A declaration below shows what a program writes against: a type&apos;s fields, a function&apos;s parameters,
          what it gives, its contracts (<C>require</C>, <C>ensure</C>) and its <C>example</C>s. <C>kavach test</C> runs
          every example, so each one is true of the code.
        </P>
        <div className="flex flex-wrap gap-2.5">
          <SrcRef file={`docs/spec/${m.file}`} label={`spec ${m.number} · ${m.title}`} />
          <SrcRef file="tools/cli/tests/stdlib.rs" label="the examples, tested" />
        </div>
        {tablesOf(m.file, m.number).length ? <H2>What the spec lists</H2> : null}
        <Tables tables={tablesOf(m.file, m.number)} />
        <Declarations m={m} />
      </>
    ),
  };
}

/** Each module's page, keyed by its course key. */
export const stdModuleDocs: Record<string, LessonDoc> = Object.fromEntries(stdModules.map((m) => [m.key, moduleDoc(m)]));

/** stdlib/README.md's row for a folder: what its files hold. */
const contents = (() => {
  const table = fileTables("stdlib/README.md").find((t) => t.header[0] === "Folder")!;
  return new Map(table.rows.map((r) => [r[0].match(/`(\w+)\/`/)?.[1] ?? "", r[2]]));
})();

export const stdlibRefDoc: LessonDoc = {
  lead: "Every standard-library module, with a page each: its types, methods and functions, their definitions and examples, and the files that declare them.",
  minutes: 4,
  body: <Overview />,
};

/** Rendered, not built when the module loads: it links to pages among the lessons. */
function Overview() {
  return (
    <>
      <P>
        A module is used with <C>use std.name</C>. Some parts are built into the compiler and runtime; the rest is
        written in Kavach, in <C>stdlib/</C>, with contracts and examples like any other code.
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Module</th>
              <th className="py-2 pr-4 font-bold">Spec</th>
              <th className="py-2 font-bold">Written in Kavach</th>
            </tr>
          </thead>
          <tbody>
            {stdModules.map((m) => (
              <tr key={m.key} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><Link href={findNode(m.key)!.href}>{m.title}</Link></td>
                <td className="py-2 pr-4 text-muted">{m.number}</td>
                <td className="py-2 text-muted">
                  {m.folders.length
                    ? m.folders.map((f) => <Cell key={f} text={contents.get(f) ?? filesOf(f).map((p) => `\`${p.split("/").pop()}\``).join(", ")} />)
                    : "Nothing: the compiler and runtime give it all"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <SrcRef file="stdlib/README.md" label="stdlib/README.md" />
    </>
  );
}
