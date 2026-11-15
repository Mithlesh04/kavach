import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith, specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Table } from "./spec-table";

// ---------- Desktop Apps ----------
//
// Spec 14.30. Its folders are made in the real home folder and its
// dialogs open on the screen, so this site shows them from the spec and
// runs only the compile error. Its two tables have no headings, so each
// is picked by its place and checked by its first row.

const FILE = "14-standard-library/30-app.md";

function AppTable({ n, first }: { n: number; first: string }) {
  const t = specTables(FILE, "14.30")[n];
  if (!t || !t.rows[0][0].includes(first)) throw new Error(`spec 14.30: table ${n} no longer starts with ${first}`);
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const appDoc: LessonDoc = {
  lead: "std.app gives a desktop program or game what it needs from the system: its own folders for saves, settings and caches, a crash report when it crashes, and the system's file and message dialogs.",
  simple: <>Every desktop program needs a place to keep its saves and settings, and every system has its own place for them. <C>std.app</C> finds the right one on Windows, Linux and macOS, writes a note when the program crashes, and shows the usual &quot;Open file&quot; box.</>,
  minutes: 5,
  before: ["standard-library/files", "standard-library/windows"],
  body: (
    <>
      <P>
        A game finds its save folder, turns on crash reports, and asks which level to load. It makes folders and shows a
        dialog, so it is shown here, not run:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.30", "app.data_dir(")} file="main.kav, from spec 14.30" />
      <H2>The program&apos;s folders</H2>
      <P>
        Each takes the program&apos;s name and gives the folder for it on this system, made with its parents if it is
        missing. So they need <C>Fs.write</C>:
      </P>
      <AppTable n={0} first="app.data_dir" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The name is a name, without <C>/</C> or <C>\</C>.</li>
        <li><C>AppError.Unavailable(reason)</C>: the system gives no such folder, as for some services with no home folder, or it can&apos;t be made.</li>
        <li>Under <C>kavach test</C>, files are fake: a folder is the path <C>/fake/data/name</C> (or <C>config</C>, <C>cache</C>) and none is made.</li>
      </ul>
      <H2>Crash reports</H2>
      <P>
        After <C>app.crash_reports(folder)</C>, a panic that ends the program also writes a file{" "}
        <C>crash-&lt;time&gt;.txt</C> into the folder: the message, where it happened, the program, Kavach&apos;s
        version, the system and the time. A player can send it to you. See{" "}
        <Link href="/docs/errors/panics/">Panics</Link>.
      </P>
      <H2>Dialogs</H2>
      <P>
        Each shows the system&apos;s own dialog and waits for the person. A dialog is a window, so it needs{" "}
        <C>Window</C>:
      </P>
      <AppTable n={1} first="app.open_file" />
      <Ex name="app/dialog-needs-window.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A dialog gives a path the person chose; reading or writing it still needs <C>Fs</C>.</li>
        <li>On a machine with no screen, a dialog gives <C>none</C>, or <C>false</C> for <C>confirm</C>.</li>
        <li>Generated tests skip functions that use <C>Window</C>, dialogs among them.</li>
      </ul>
      <Note>Not yet: a call stack in crash reports, an icon for Windows executables, and file type filters in <C>open_file</C>.</Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.30 · std.app" />
        <SrcRef file="tests/conformance/14-standard-library/dialogs-need-window.kav" label="Window, tested" />
        <SrcRef file="tools/cli/tests/app.rs" label="folders and crash reports, tested" />
        <SrcRef file="stdlib/app/app.kav" label="app.kav" />
        <SrcRef file="docs/decisions/0118-app-folders-crash-reports-and-dialogs.md" label="decision 0118" />
      </div>
    </>
  ),
};
