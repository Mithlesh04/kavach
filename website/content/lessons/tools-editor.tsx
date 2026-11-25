import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { H2, P, SpecCode } from "./parts";
import { Table } from "./spec-table";

// ---------- Editor Setup ----------
//
// The VS Code extension (editors/vscode) and what `kavach lsp` shows in it:
// spec 10.5 and 13.12, mistakes shown while you type, with their quick fixes.

/** Spec 13.12's table of mistakes the editor shows as you type. */
function mistakes() {
  const t = specTables("13-ai-tooling.md", "13.12").find((x) => x.header[0] === "Mistake");
  if (!t) throw new Error("spec 13.12: no Mistake table");
  return t;
}

export const editorDoc: LessonDoc = {
  lead: "The Kavach extension for VS Code, Cursor and other VS Code-based editors colors code and shows errors as you type.",
  simple: <>Like the spell checker in a word processor, the extension underlines mistakes while you type, and colors your code so it is easier to read.</>,
  minutes: 5,
  before: ["start/install"],
  body: (
    <>
      <H2>What it gives</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Colors for <C>.kav</C> files, and for <C>```kavach</C> blocks inside Markdown.</li>
        <li>Errors and warnings as you type, with the same messages as <C>kavach check</C>. The check runs on its own, so hover, completion and colors answer at once while a file is checked, and a check of text you have since changed is never shown.</li>
        <li>Hover to see a value&apos;s type; <C>F12</C> to go to where a name is declared.</li>
        <li>Completion: the names in scope, and after a <C>.</C> the value&apos;s fields and methods.</li>
        <li>Names colored by where they come from: what a <C>use</C> brings in is lavender, the language&apos;s own methods (<C>xs.push(2)</C>, <C>s.len()</C>) gold, and your own methods your theme&apos;s color. So a call shows whether it is the language&apos;s or the program&apos;s.</li>
        <li>
          Quick fixes:
          <ul className="m-0 mt-1 flex list-[circle] flex-col gap-1 pl-5">
            <li>&quot;Mark <C>helper</C> as a <C>gpu fn</C>&quot;, when a <C>gpu fn</C> calls a plain function that fits the GPU&apos;s rules.</li>
            <li>Every fix that <Link href="/docs/tools/the-kavach-command/fix/">kavach fix</Link> makes, one at a time: &quot;Remove this arm&quot;, &quot;Remove this branch&quot;, &quot;Write <C>==</C>&quot;, &quot;Declare it with <C>let</C>&quot;, &quot;Add <C>uses Fs.read</C>&quot; and the rest.</li>
            <li>Two that may not be what you meant, so they are offered, never made for you: &quot;Change to <C>total</C>&quot; on a misspelled name, and &quot;Change the <C>total</C> declared above&quot; on a <Link href="/docs/basics/variables/changing-a-value/"><C>let</C> that hides another</Link>.</li>
          </ul>
          They find the code in the compiled program, not by matching text, so each changes exactly what its message is about.
        </li>
        <li>Comment toggling, bracket matching, and <C>{"///"}</C> continuing on Enter.</li>
      </ul>
      <H2>Mistakes shown while you type</H2>
      <P>
        The extension shows every error and warning <C>kavach check</C> gives, as you type, before you save. So mistakes
        that would otherwise show up only in a test, or in production, show up in the editor:
      </P>
      <Table header={mistakes().header} rows={mistakes().rows} wrapFirst />
      <SpecCode file="13-ai-tooling.md" number="13.12" text={"type Guard: Drop"} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Each message carries its code, so an editor or an AI tool acts on the kind of mistake. The codes are listed in <Link href="/docs/reference/compiler-messages/">Compiler Messages</Link>.</li>
        <li>A fix with one right answer, such as adding <C>Fs</C> to <C>uses</C>, is also what <C>kavach fix</C> applies. A fix that changes what the code does, such as naming a <C>_</C> value <C>_guard</C>, which changes when it drops, is offered only.</li>
        <li><strong className="text-ink">Checked file by file.</strong> Opening a file checks the whole program. After that, an edit inside the bodies of functions checks that file alone, against the other files&apos; declarations, so a big project isn&apos;t checked again for each change. Changing a declaration, or saving, checks it all again.</li>
      </ul>
      <H2>Install it</H2>
      <P>The extension isn&apos;t on the Marketplace yet: it is built from the repository, with no npm needed.</P>
      <Terminal
        commands
        session={
          "cargo build --release -p kavach-cli\nsh scripts/package-vscode.sh\ncode --install-extension .kavach/kavach-vscode-0.2.0.vsix"
        }
      />
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li>The first line builds <C>kavach</C> (it needs Rust). On Windows, package with <C>powershell -File scripts/package-vscode.ps1</C>.</li>
        <li>If <C>kavach</C> isn&apos;t on your <C>PATH</C>, set <strong className="text-ink">Kavach: Path</strong> in the settings to the program the first line built.</li>
        <li>Reload the window (<C>Ctrl+Shift+P</C>, <strong className="text-ink">Developer: Reload Window</strong>).</li>
        <li>Check it: write <C>const x: Int = &quot;text&quot;</C> in a <C>.kav</C> file, and a red line appears.</li>
      </ol>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="editors/vscode/README.md" label="the extension's README" />
        <SrcRef file="docs/spec/13-ai-tooling.md" label="spec 13.12 · the language server" />
      </div>
    </>
  ),
};
