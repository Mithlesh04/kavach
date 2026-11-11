import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specBlock, specTables, type SpecTable } from "@/lib/spec-tables";
import { H2, P } from "./parts";
import { Cell, Table } from "./spec-table";

const SPEC = "10-modules-and-packages.md";

// ---------- Project Layout ----------

const layoutRows: [string, string, string][] = [
  ["kavach.toml", "The project's name, version and settings", "yes"],
  ["kavach.lock", "Exact versions of the packages used (written by Kavach)", "yes"],
  ["src/", "Your code; main.kav for a program, lib.kav for a library", "yes"],
  ["src/orders.test.kav", "The hand-written tests of orders.kav, beside it, if it has any", "yes"],
  ["tests/", "Tests that tell a story across many modules, if any", "yes"],
  ["tests/found/", "Failing inputs kavach test found, replayed on every run (written by Kavach)", "yes"],
  ["kavach_modules/", "Fetched dependencies, one folder per package (written by Kavach; kavach.lock is enough to fetch them again)", "never"],
  [".kavach/", "Build output and caches: the executable lands in .kavach/bin/", "never"],
];

const created = loadProject("tools/settings", ["kavach.toml", "src/main.kav"]);
const typo = loadProject("tools/settings-typo.fails", ["kavach.toml"]);

export const layoutDoc: LessonDoc = {
  lead: "A project is a folder: kavach.toml at the top, your code in src/, and build output in .kavach/, never committed.",
  simple: <>A project folder is like a well-ordered desk: the settings sheet, <C>kavach.toml</C>, on top, your code in the <C>src/</C> drawer, and what Kavach makes for you in <C>.kavach/</C>, which you don&apos;t save with your code.</>,
  minutes: 4,
  before: ["tools/modules"],
  body: (
    <>
      <CodeBlock code={specBlock(SPEC, "10.3")} file="my-app/" lang="text" />
      <H2>What each part is, and what to commit</H2>
      <Table header={["Path", "What it is", "Commit it?"]} rows={layoutRows} />
      <H2>Start one</H2>
      <P><C>kavach new</C> makes the folder, a first <C>kavach.toml</C>, a <C>src/main.kav</C>, and a <C>.gitignore</C> that leaves <C>.kavach/</C> out:</P>
      <Terminal commands session={"kavach new my-app\ncd my-app\nkavach run"} />
      <P>Inside a project, <C>kavach run</C>, <C>build</C>, <C>test</C> and <C>check</C> need no file name: they use <C>src/main.kav</C>.</P>
      <div><SrcRef file={`docs/spec/${SPEC}`} label="spec 10.3 · project layout" /></div>
    </>
  ),
};

// ---------- Settings ----------

export const settingsDoc: LessonDoc = {
  lead: "Every setting a project can have, with its values and its default: kavach.toml, then environment variables.",
  simple: <><C>kavach.toml</C> is like a form with boxes for your project&apos;s choices. Leave a box empty and the usual choice is used; write in a box that doesn&apos;t exist, and Kavach tells you, so a typo is never missed.</>,
  minutes: 6,
  before: ["tools/project-layout"],
  body: (
    <>
      <P>A project&apos;s settings live in <C>kavach.toml</C>, next to <C>src/</C>. Leave a setting out and its default applies:</P>
      <CodeBlock code={created.files[0].code} file="kavach.toml" lang="toml" />
      <P>
        A key or section that isn&apos;t in the tables below is an error, and the message lists the names the section
        does have, so a typo can&apos;t be ignored silently:
      </P>
      <CodeBlock code={typo.files[0].code} file="kavach.toml" lang="toml" output={typo.output} outcome="refused" />
      <SettingsTables />
      <P>Paths in <C>kavach.toml</C> are relative to the folder it is in.</P>
      <H2>Environment variables</H2>
      <P>Set in the shell before running a program, for one run, instead of in a file:</P>
      <Table {...envTable(0)} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${SPEC}`} label="spec 10.11 · settings reference" />
        <SrcRef file="compiler/driver/tests/settings.rs" label="the test that keeps this table true" />
      </div>
    </>
  ),
  deeper: (
    <>
      <P>For work on the compiler itself: these print what a pass did, or turn one off.</P>
      <Table {...envTable(1)} />
    </>
  ),
};

function envTable(i: number): SpecTable {
  const tables = specTables(SPEC, "10.11").filter((t) => t.heading === "Environment variables");
  if (!tables[i]) throw new Error(`spec 10.11: no environment table ${i}`);
  return tables[i];
}

/** The kavach.toml table of spec 10.11, one small table per section. */
function SettingsTables() {
  const toml = specTables(SPEC, "10.11").find((t) => t.heading === "kavach.toml");
  if (!toml || toml.header.join("|") !== "Section|Key|Values|Default|Spec") {
    throw new Error("spec 10.11: the kavach.toml table changed shape; update the Settings page");
  }
  const sections = [...new Set(toml.rows.map((r) => r[0]))];
  return (
    <>
      {sections.map((s) => (
        <section key={s} aria-label={s} className="flex flex-col gap-2">
          <h3 className="m-0 font-mono text-[17px] font-bold"><Cell text={s} /></h3>
          <Table header={["Key", "Values", "Default", "Spec"]} rows={toml.rows.filter((r) => r[0] === s).map((r) => r.slice(1))} />
        </section>
      ))}
    </>
  );
}

// ---------- Editions ----------

const edition = loadProject("tools/edition.fails", ["kavach.toml"]);

export const editionsDoc: LessonDoc = {
  lead: "Code that compiles today keeps compiling. A change that would break old code comes in a new edition, which each package chooses.",
  simple: <>An edition is like the rules of a board game printed on its box: a newer box can change the rules, but your old game is still played by the rules on its own box.</>,
  minutes: 2,
  before: ["tools/settings"],
  body: (
    <>
      <P>
        A package names its edition in <C>kavach.toml</C>. <C>kavach new</C> writes the newest one:
      </P>
      <CodeBlock code={specBlock(SPEC, "10.8")} file="kavach.toml" lang="toml" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Packages on different editions work together in one program.</li>
        <li>Editions come out rarely, every few years, and only with a tool that upgrades code to the new one.</li>
        <li><C>2026</C> is the first and, so far, the only edition.</li>
      </ul>
      <P>An edition Kavach doesn&apos;t have is an error that names the ones it has:</P>
      <CodeBlock code={edition.files[0].code} file="kavach.toml" lang="toml" output={edition.output} outcome="refused" />
      <SrcRef file={`docs/spec/${SPEC}`} label="spec 10.8 · editions" />
    </>
  ),
};
