import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { moduleDecls } from "@/lib/outline";
import { fileFigure, specTables } from "@/lib/spec-tables";
import { H2, P } from "./parts";
import { teacher } from "./reference";
import { Cell } from "./spec-table";

// ---------- Reference › Built-in Types ----------
//
// Every type a program can use without `use`: the compiler's own list
// (compiler/resolver/src/prelude.rs), and the types written in Kavach
// that every program gets (Json, Taken). What each one is comes from the
// spec, read with a pattern that fails the build when the text changes;
// where each is defined is the line that names it.

const PRELUDE = "compiler/resolver/src/prelude.rs";
const repo = path.join(process.cwd(), "..");

/** The prelude's list of type names, in its order. */
const preludeTypes: string[] = (() => {
  const list = fileFigure(PRELUDE, /pub\(crate\) const TYPES: &\[&str\] = &\[([\s\S]*?)\];/);
  return [...list.matchAll(/"(\w+)"/g)].map((m) => m[1]);
})();

/** The line of prelude.rs that names a type. */
function preludeLine(name: string): number {
  const i = fs.readFileSync(path.join(repo, PRELUDE), "utf8").split(/\r?\n/).findIndex((l) => l.trim() === `"${name}",`);
  if (i < 0) throw new Error(`Reference: ${PRELUDE} doesn't name ${name}`);
  return i + 1;
}

type Entry = { names: string[]; says: string; number: string; file: string; defined?: { file: string; line: number } };

/** A spec sentence: the first capture of `pattern` in a spec file. */
const said = (file: string, pattern: RegExp) => fileFigure(`docs/spec/${file}`, pattern);

/** A row of a spec table whose function gives the type: "`net.bind(address) -> UdpSocket ! NetError`: A UDP socket at `address`". */
function givenBy(file: string, signature: string): string {
  const esc = signature.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return said(file, new RegExp(`^\\| (\`${esc}\` \\| [^|]*?) \\|`, "m")).replace(" | ", ": ");
}

const NET = "14-standard-library/10-net.md";

const MEMORY = "06-memory.md";
const TASKS = "09-concurrency.md";

/** Groups of types, each type with the spec's words for it. */
function groups(): { title: string; entries: Entry[] }[] {
  const json = moduleDecls(["stdlib/json/json.kav"]);
  const fromStd = (name: string) => {
    const d = json.find((x) => x.name === name)!;
    return { file: d.file, line: d.line };
  };
  const basics = specTables("02-types/01-built-in-types.md", "2.1")[0].rows.map(([cell, meaning]) => {
    const names = [...cell.matchAll(/`(\w+)`/g)].map((m) => m[1]);
    // Json is written in Kavach, in std.json, which every program gets.
    return { names, says: meaning, number: "2.1", file: "02-types/01-built-in-types.md", defined: names[0] === "Json" ? fromStd("Json") : undefined };
  });
  const collections = specTables("02-types/02-collections.md", "2.2")[0].rows.map(([type, short, literal]) => ({
    names: [type.match(/`(\w+)/)![1]],
    says: `Written ${short === "–" ? type : short}; a literal: ${literal}`,
    number: "2.2",
    file: "02-types/02-collections.md",
  }));
  const taken = moduleDecls(["stdlib/tasks/tasks.kav"]).find((d) => d.name === "Taken")!;
  return [
    { title: "Numbers, text and values", entries: basics },
    { title: "Collections", entries: collections },
    {
      title: "Errors",
      entries: [
        { names: ["Error"], says: said("05-errors.md", /(For scripts and application code that just wants to stop on any error, `! Error` accepts every error type)/), number: "5.5", file: "05-errors.md" },
        { names: ["ParseError"], says: said("02-types/01-built-in-types.md", /(It \*\*can fail\*\* with a `ParseError`, so it takes `try` or `catch`\. `Int\("23sd"\)` fails, as does `Int\("300"\)` into `Int8`\.)/), number: "2.1", file: "02-types/01-built-in-types.md" },
        { names: ["JsonError"], says: json.find((d) => d.name === "JsonError")!.doc, number: "2.12", file: "02-types/12-json.md", defined: fromStd("JsonError") },
      ],
    },
    {
      title: "Tasks and channels",
      entries: [
        { names: ["Task"], says: said(TASKS, /(`spawn` starts a task and returns a `Task<T>`\. `\.wait\(\)` gets its result\.)/), number: "9.1", file: TASKS },
        { names: ["Channel", "Sender", "Receiver"], says: said(TASKS, /(Channels pass values between tasks\. `Channel<T>\(capacity: n\)` gives its two ends, a `Sender<T>` and a `Receiver<T>`)/), number: "9.4", file: TASKS },
        { names: ["Taken"], says: taken.doc, number: "9.4", file: TASKS, defined: { file: taken.file, line: taken.line } },
      ],
    },
    {
      title: "Shared state and memory",
      entries: [
        { names: ["Shared"], says: said(MEMORY, /(`Shared<T>` is the only way to get shared, mutable state\.)/), number: "6.2", file: MEMORY },
        { names: ["Weak"], says: said(MEMORY, /(`Weak<T>` points to a `Shared<T>` without keeping it alive\.)/), number: "6.2", file: MEMORY },
        { names: ["Ptr"], says: said(MEMORY, /(A `Ptr<T>` is the address of `T` values in memory Kavach doesn't manage, as C has it\.)/), number: "6.4", file: MEMORY },
      ],
    },
    {
      title: "Files, text and networks",
      entries: [
        { names: ["Lines", "Chunks"], says: said("14-standard-library/03-fs.md", /(`fs\.lines` gives a reader of type `Lines` and `fs\.chunks` one of type `Chunks`)/), number: "14.3", file: "14-standard-library/03-fs.md" },
        { names: ["Regex", "RegexError"], says: givenBy("14-standard-library/08-regex.md", "regex.new(pattern) -> Regex ! RegexError"), number: "14.8", file: "14-standard-library/08-regex.md" },
        { names: ["CliError"], says: givenBy("14-standard-library/06-io-and-cli.md", "cli.parse<T>() -> T ! CliError"), number: "14.6", file: "14-standard-library/06-io-and-cli.md" },
        { names: ["Listener"], says: givenBy(NET, "net.listen(address) -> Listener ! NetError"), number: "14.10", file: NET },
        { names: ["Connection"], says: givenBy(NET, "net.connect(address) -> Connection ! NetError"), number: "14.10", file: NET },
        { names: ["UdpSocket"], says: givenBy(NET, "net.bind(address) -> UdpSocket ! NetError"), number: "14.10", file: NET },
      ],
    },
    {
      title: "Interfaces the language relies on",
      entries: [
        { names: ["Display"], says: said("01-basics.md", /(A type can choose its own form by implementing the `Display` interface \(`fn display\(self\) -> String`\)\.)/), number: "1.11", file: "01-basics.md" },
        { names: ["Drop"], says: said(MEMORY, /(Types that hold resources can run cleanup by implementing `Drop`)/), number: "6.1", file: MEMORY },
        { names: ["Comparable"], says: said("02-types/09-generics.md", /(`<T: Comparable>` means "any type `T` that implements the interface `Comparable`"\.)/), number: "2.9", file: "02-types/09-generics.md" },
      ],
    },
  ];
}

export const typesRefDoc: LessonDoc = {
  lead: "Every type a program can use without use: numbers, text, collections, errors, tasks and channels, shared state, and the interfaces the language relies on. Each with what it is and where it is defined.",
  minutes: 6,
  body: <Types />,
};

/** Rendered, not built when the module loads: the links need every lesson, and this one is among them. */
function Types() {
  const all = groups();
  const listed = new Set(all.flatMap((g) => g.entries.flatMap((e) => e.names)));
  const missing = preludeTypes.filter((t) => !listed.has(t) && t !== "Self");
  if (missing.length) throw new Error(`Reference: ${PRELUDE} has types no group lists: ${missing.join(", ")}`);
  return (
    <>
      <P>
        These need no <C>use</C>. Their methods are listed in{" "}
        <Link href="/docs/reference/built-in-methods/">Built-in Methods</Link>, and the types a module adds on its{" "}
        <Link href="/docs/reference/standard-library/">module&apos;s page</Link>. Each type links to the course page that
        teaches it with examples that run.
      </P>
      {all.map((g) => (
        <div key={g.title} className="flex flex-col gap-3">
          <H2>{g.title}</H2>
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-[15px]">
              <thead>
                <tr className="border-b border-line text-left text-soft">
                  <th className="py-2 pr-4 font-bold">Type</th>
                  <th className="py-2 pr-4 font-bold">What it is</th>
                  <th className="py-2 font-bold">Learn · defined in</th>
                </tr>
              </thead>
              <tbody>
                {g.entries.map((e) => {
                  const page = teacher(e.number);
                  const at = e.defined ?? (preludeTypes.includes(e.names[0]) ? { file: PRELUDE, line: preludeLine(e.names[0]) } : undefined);
                  return (
                    <tr key={e.names.join()} className="border-b border-line align-top">
                      <td className="py-2 pr-4">{e.names.map((n) => <div key={n}><C>{n}</C></div>)}</td>
                      <td className="py-2 pr-4 text-muted"><Cell text={e.says} /></td>
                      <td className="py-2 text-muted">
                        <div className="flex flex-col items-start gap-1.5">
                          {page ? <Link href={page.href}>{page.title}</Link> : null}
                          <SrcRef file={`docs/spec/${e.file}`} label={`spec ${e.number}`} />
                          {at ? <SrcRef file={at.file} line={at.line} label={`${at.file.split("/").pop()}:${at.line}`} /> : null}
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      ))}
    </>
  );
}
