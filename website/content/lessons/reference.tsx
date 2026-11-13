import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { course } from "@/lib/course";
import { findNode } from "@/lib/nav";
import { hasContent } from "@/lib/site";
import { fileFigure, specBlock, specTables, type SpecTable } from "@/lib/spec-tables";
import { H2, Note, P } from "./parts";
import { Cell, Table } from "./spec-table";

// ---------- Reference ----------
//
// Lookup pages read from the spec at build time, never typed: a change to
// the spec changes these pages by itself. Each links to the course page
// that teaches the topic, found by its spec number in lib/course.ts.

const STDLIB = "14-standard-library";

/** The course page outside the Reference that teaches spec section `number`, if one is written. */
function teacher(number: string): { href: string; title: string } | undefined {
  for (const part of course) {
    if (part.slug === "reference") continue;
    for (const page of part.pages) {
      const key = `${part.slug}/${page.slug}`;
      if (page.spec === number && hasContent(key)) return { href: findNode(key)!.href, title: page.title };
    }
  }
  return undefined;
}

/** "Spec N · learn it in …"; `alone` for a section many pages teach, such as 14.2. */
function Learn({ number, keyed, alone = false }: { number: string; keyed?: string; alone?: boolean }) {
  const node = keyed ? findNode(keyed) : undefined;
  if (keyed && !node) throw new Error(`Reference: no course page ${keyed}`);
  const page = node ? { href: node.href, title: node.title } : alone ? undefined : teacher(number);
  return (
    <P>
      Spec {number}
      {page ? (
        <>
          {" "}· learn it in <Link href={page.href}>{page.title}</Link>
        </>
      ) : null}
      .
    </P>
  );
}

/** Spec tables, each under its heading or the sentence that introduces it. */
function Tables({ tables }: { tables: SpecTable[] }) {
  return (
    <>
      {tables.map((t, i) => (
        <div key={i} className="flex flex-col gap-2">
          {t.caption ? <p className="m-0 text-[15px] font-bold text-soft"><Cell text={t.caption} /></p> : null}
          <Table header={t.header} rows={t.rows} wrapFirst />
        </div>
      ))}
    </>
  );
}

export const keywordsRefDoc: LessonDoc = {
  lead: "Every keyword of Kavach, and the words that are keywords in one place only.",
  minutes: 2,
  body: (
    <>
      <Learn number="1.6" keyed="basics/syntax/keywords" />
      <div className="flex flex-wrap gap-2">
        {specBlock("01-basics.md", "1.6").split(/\s+/).filter(Boolean).map((k) => <C key={k}>{k}</C>)}
      </div>
      <P>A keyword can&apos;t be a name. These are keywords only where the table says, and ordinary names everywhere else:</P>
      <Tables tables={specTables("01-basics.md", "1.6")} />
      <SrcRef file="docs/spec/01-basics.md" label="spec 1.6 · keywords" />
    </>
  ),
};

export const operatorsRefDoc: LessonDoc = {
  lead: "Every operator, by group, as the spec lists them.",
  minutes: 2,
  body: (
    <>
      <Learn number="1.8" keyed="basics/operators" />
      <Tables tables={specTables("01-basics.md", "1.8")} />
      <SrcRef file="docs/spec/01-basics.md" label="spec 1.8 · operators" />
    </>
  ),
};

export const literalsRefDoc: LessonDoc = {
  lead: "How each kind of value is written in code: numbers, text, characters and the rest.",
  minutes: 2,
  body: (
    <>
      <Learn number="1.7" keyed="basics/data-types" />
      <Tables tables={specTables("01-basics.md", "1.7")} />
      <SrcRef file="docs/spec/01-basics.md" label="spec 1.7 · literals" />
    </>
  ),
};

export const methodsRefDoc: LessonDoc = {
  lead: "The methods of numbers, text, lists, sets and maps, which every program has without a use.",
  minutes: 4,
  body: (
    <>
      <Learn number="14.2" alone />
      <Tables tables={specTables(`${STDLIB}/02-built-in-methods.md`, "14.2")} />
      <SrcRef file={`docs/spec/${STDLIB}/02-built-in-methods.md`} label="spec 14.2 · built-in methods" />
    </>
  ),
};

/** The standard library's modules, one spec file each, after 14.1 (how it is organized) and 14.2 (built-in methods). */
const modules = fs
  .readdirSync(path.join(process.cwd(), "..", "docs", "spec", STDLIB))
  .filter((f) => /^\d\d-/.test(f) && !/^0[12]-/.test(f))
  .sort()
  .map((file) => {
    const [, number, name] = fs
      .readFileSync(path.join(process.cwd(), "..", "docs", "spec", STDLIB, file), "utf8")
      .match(/^# (14\.\d+) (.+)/)!;
    return { file, number, name: name.trim() };
  });

export const stdlibRefDoc: LessonDoc = {
  lead: "Every function and type of every standard-library module, read from the spec: one place to look one up.",
  minutes: 15,
  body: (
    <>
      <P>
        Each module below is the spec&apos;s own table, so it lists exactly what the spec says. Its course page teaches it
        with examples that run.
      </P>
      <ul className="m-0 flex flex-wrap gap-x-4 gap-y-1 pl-0 list-none">
        {modules.map((m) => (
          <li key={m.file}><a href={`#${`${m.number} ${m.name}`.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "")}`}><C>{m.name}</C></a></li>
        ))}
      </ul>
      {modules.map((m) => (
        <div key={m.file} className="flex flex-col gap-3">
          <H2>{`${m.number} ${m.name}`}</H2>
          <Learn number={m.number} />
          <Tables tables={specTables(`${STDLIB}/${m.file}`, m.number)} />
        </div>
      ))}
    </>
  ),
};

const COMMANDS = "10-modules-and-packages/05-command-line.md";

export const commandsRefDoc: LessonDoc = {
  lead: "Every kavach command the spec describes, with what it does.",
  minutes: 3,
  body: (
    <>
      <Learn number="10.5" keyed="tools/the-kavach-command" />
      <Tables tables={specTables(COMMANDS, "10.5")} />
      <Note><Cell text={fileFigure(`docs/spec/${COMMANDS}`, /^(Not built yet:[^\n]*)/m)} /></Note>
      <P>
        Settings in <C>kavach.toml</C> and the environment variables Kavach reads are listed in{" "}
        <Link href={findNode("tools/settings")!.href}>Settings</Link>.
      </P>
      <SrcRef file={`docs/spec/${COMMANDS}`} label="spec 10.5 · command line" />
    </>
  ),
};
