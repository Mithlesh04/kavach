import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Files ----------

export const filesDoc: LessonDoc = {
  lead: "std.fs reads and writes files and folders. Paths are text, relative to the folder the program runs in.",
  simple: <>Files are like papers kept in folders on your computer. With <C>std.fs</C>, a program can read a paper, write a new one, or make folders, so its work is still there after it stops.</>,
  minutes: 5,
  before: ["errors/errors", "basics/for-loop"],
  body: (
    <>
      <Ex name="files/sub-write-read" />
      <H2>Every function</H2>
      <ModuleTable file="03-fs.md" number="14.3" first="Function" />
      <H2>When it fails</H2>
      <P>
        A file operation can fail: the file isn&apos;t there, or you may not read it. It then gives an{" "}
        <C>FsError</C>: <C>NotFound(path)</C>, <C>Denied(path)</C> or <C>Failed(path, reason)</C>. Match on it to
        handle each case:
      </P>
      <Ex name="files/sub-missing" />
      <Note>
        Reading or writing files needs the <C>Fs</C> capability. A <C>pub</C> function says so with{" "}
        <C>uses Fs</C>, or only <C>uses Fs.read</C>: see <Link href="/docs/agents/capabilities/narrower-file-access/">Narrower file access</Link>.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Write <C>hello.txt</C> with the text <C>Namaste</C>, read it back and print it.</>,
    answer: <P><C>try fs.write_text(&quot;hello.txt&quot;, &quot;Namaste&quot;)</C>, then <C>print(try fs.read_text(&quot;hello.txt&quot;))</C>, in a <C>main</C> marked <C>! Error</C>.</P>,
  },
  quiz: [
    {
      type: "choice",
      prompt: <>The power goes off while <C>fs.write_text</C> is replacing a file. What is in the file afterwards?</>,
      options: ["Either all of the old text or all of the new, never half", "Half of the new text", "Nothing: the file is empty"],
      answer: 0,
      explain: "write_text writes a temporary file and then renames it, so a crash never leaves half a file.",
    },
  ],
};

export const readingDoc: LessonDoc = {
  lead: "fs.read_text reads a whole file; fs.lines reads it one line at a time, for files of any size.",
  simple: <>Reading a whole file is like reading a whole letter at once. For a very long file, read it one line at a time, like a book page by page, so the program never has to hold all of it at once.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="files/sub-lines" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>fs.lines</C> keeps only the line being looked at in memory, so a file of many gigabytes is fine.</li>
        <li>Lines come without their ending, <C>\n</C> or <C>\r\n</C>.</li>
        <li>A read that fails halfway goes to the <C>try</C> in the loop&apos;s header, so half a file is never taken for a whole one.</li>
      </ul>
      <P>Files that aren&apos;t text are read as <C>Bytes</C>, whole or in pieces:</P>
      <Ex name="files/bytes" />
      <H2>Big pieces are read ahead</H2>
      <P>
        <C>fs.chunks</C> keeps only the piece being looked at in memory. With pieces of 1 MB or more, the next piece is
        in memory too: a native program reads it on a helper thread while the loop works on this one, so the disk and
        the work overlap. There is nothing to write. The interpreter, plain <C>kavach run</C>, doesn&apos;t do this.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The file&apos;s bytes and the order of the pieces stay the same, and an error comes at the same point of the loop.</li>
        <li>A change the program makes to the file during the loop may not show in the piece after.</li>
      </ul>
      <P>
        What it gains is measured on <Link href="/docs/speed/gpu/can-i-use-it-on-the-gpu/">Can I use it on the GPU?</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/03-fs.md" label="spec 14.3 · files" />
        <SrcRef file="tests/conformance/14-standard-library/big-chunks-read-the-whole-file-in-order.kav" label="big pieces, tested" />
      </div>
    </>
  ),
};

export const writingDoc: LessonDoc = {
  lead: "fs.write_text replaces a file in one step; fs.append_text adds to its end.",
  simple: <>Writing replaces the whole page with new text; appending adds to the end, like a new entry at the bottom of a diary. If the computer crashes while writing, you keep the old text or the new, never half.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="files/append" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>append_text</C> creates the file if needed, then adds to its end: good for logs.</li>
        <li><C>write_text</C> replaces the whole file, atomically: a crash leaves the old text or the new, never half.</li>
        <li><C>fs.write_durable</C> also waits until the data is on the disk, so even a power cut keeps it. Databases build on it.</li>
      </ul>
    </>
  ),
};

export const foldersDoc: LessonDoc = {
  lead: "fs.create_dir makes a folder and its parents; fs.list gives the names in a folder, sorted.",
  simple: <>Folders are like boxes that hold files and other boxes. You can make a box inside boxes that don&apos;t exist yet, in one step, and list the names of what is inside, in sorted order.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="files/folders" />
      <P>
        <C>fs.remove</C> deletes a file or an empty folder, and <C>fs.exists</C> says whether something is there.
        Paths are relative to the folder the program runs in.
      </P>
      <H2>Copy, rename and remove a whole folder</H2>
      <Ex name="files/move" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>fs.copy</C> copies a file; <C>fs.rename</C> moves or renames a file or a folder.</li>
        <li><C>fs.remove_all</C> deletes a folder with everything in it.</li>
        <li>Under <C>{'uses Fs("folder")'}</C>, both paths of <C>copy</C> and <C>rename</C> must be inside that folder, so a file can&apos;t be carried out of it.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/14-standard-library/fs-info-and-permissions.kav" label="tested" />
        <SrcRef file="tests/conformance/08-capabilities/rename-and-copy-stay-inside-the-folder.kav" label="both paths inside the folder, tested" />
      </div>
    </>
  ),
};

export const fileInfoDoc: LessonDoc = {
  lead: "fs.info gives a file's kind, size, time of change and permissions; fs.set_read_only and fs.set_permissions change them.",
  simple: <>Every file has a label: is it a file or a folder, how big it is, when it last changed, and who may change it. <C>fs.info</C> reads the label, and two functions change who may write the file.</>,
  minutes: 3,
  before: ["standard-library/files"],
  body: (
    <>
      <Ex name="files/info" />
      <CodeBlock code={specBlockWith("14-standard-library/03-fs.md", "14.3", "pub type FileInfo")} file="std.fs" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>permissions</C> are the Unix bits, written in octal: <C>0o644</C> lets the owner read and write, and everyone else read; <C>0o755</C> lets anyone run the file.</li>
        <li>A mode outside <C>0o0</C> to <C>0o7777</C> fails, as the last line shows.</li>
        <li><strong className="text-ink">On Windows,</strong> files have no Unix bits. <C>permissions</C> is <C>0o444</C> for a read-only file and <C>0o666</C> otherwise (<C>0o555</C> and <C>0o777</C> for a folder), as Go and Rust report them. <C>set_permissions</C> keeps only the owner&apos;s write bit, <C>0o200</C>. A drive that keeps no Unix bits, such as a Windows drive mounted in Linux, keeps only read-only too.</li>
        <li><C>fs.info</C> reads, so it needs <C>Fs.read</C>; the others change files and need <C>Fs.write</C>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/03-fs.md" label="spec 14.3 · file details and permissions" />
        <SrcRef file="tests/conformance/14-standard-library/fs-info-and-permissions.kav" label="tested" />
      </div>
    </>
  ),
};

// ---------- Command-line Programs ----------

const args = loadProject("cli/args", ["src/main.kav"]);
const parsed = loadProject("cli/parse", ["src/main.kav"]);
const commands = loadProject("cli/commands", ["src/main.kav"]);

export const cliDoc: LessonDoc = {
  lead: "A command-line program reads the words typed after its name. cli.args gives them as a list; cli.parse turns them into a record.",
  simple: <>When you type <C>greet Asha Pune</C>, the words after the program&apos;s name are its arguments, like instructions written on an order slip. The program reads them to know what to do.</>,
  minutes: 4,
  before: ["standard-library/files"],
  body: (
    <>
      <CodeBlock code={args.files[0].code} file="src/main.kav" />
      <Terminal session={args.output ?? ""} />
      <P>
        <C>cli.args()</C> gives the arguments without the program&apos;s own name. Reading them needs the{" "}
        <C>Env</C> capability, which <C>main</C> has. Arguments after <C>--</C> go to the program too, without the{" "}
        <C>--</C>, in every way of running it: the last run above is native code.
      </P>
      <H2>Every function</H2>
      <ModuleTable file="06-io-and-cli.md" number="14.6" first="Function" />
      <Note>
        <C>kavach build</C> makes the program an executable of its own, run as <C>./greet Asha Pune</C>, with the
        same arguments.
      </Note>
    </>
  ),
};

export const argumentsDoc: LessonDoc = {
  lead: "Describe the options as a record, and cli.parse reads them, checks their types, and writes the help text for you.",
  simple: <>Think of a form with labelled boxes, such as <C>--limit</C>: you say which boxes exist and what kind of answer each takes. Kavach fills them from what the user typed, and complains if an answer is the wrong kind.</>,
  minutes: 4,
  body: (
    <>
      <CodeBlock code={parsed.files[0].code} file="src/main.kav" />
      <Terminal session={parsed.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A field <C>limit</C> is the option <C>--limit</C>; an <C>_</C> in a name is written <C>-</C>.</li>
        <li>A field with a default may be left out. One without a default, like <C>input</C>, is required.</li>
        <li>A <C>Bool</C> is a flag with no value: <C>--verbose</C>.</li>
        <li><C>--help</C>, or a mistake, gives an error whose text is the usage.</li>
      </ul>
    </>
  ),
};

export const commandsDoc: LessonDoc = {
  lead: "Parse into a union, and each case is a command with its own options, like git add and git log.",
  simple: <>Some programs do several different jobs, like a post office with separate counters for letters and parcels. Each command is one counter, with its own form of options.</>,
  minutes: 3,
  body: (
    <>
      <CodeBlock code={commands.files[0].code} file="src/main.kav" />
      <Terminal session={commands.output ?? ""} />
      <P>
        The case <C>AddItem</C> is the command <C>add-item</C>. <C>--help</C> before a command lists every command;
        after one, it shows that command&apos;s options.
      </P>
    </>
  ),
};
