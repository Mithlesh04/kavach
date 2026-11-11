import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specBlock } from "@/lib/spec-tables";
import { H2, Note, P } from "./parts";

// The commands `kavach --help` lists today; spec 10.5 has the rest, which
// come later.
const today: [cmd: string, does: string][] = [
  ["kavach new my-app", "make a project: kavach.toml, src/main.kav and a .gitignore"],
  ["kavach run [file.kav] [-- args]", "check and run a file, or the project's src/main.kav"],
  ["kavach run --dry-run", "run without making any capability call"],
  ["kavach run --native", "compile to machine code first"],
  ["kavach build [file.kav] [-o out]", "write an executable"],
  ["kavach build --compress [=max]", "write a smaller executable, packed without losing anything; it unpacks itself when it starts"],
  ["kavach build --lib lib.kav", "write a shared library and a C header, for other languages"],
  ["kavach add <name> --path <folder>", "add a dependency from a folder; --git <url> with --tag, --branch or --rev for a repository, --unsafe to allow one that uses unsafe code"],
  ["kavach update", "fetch every git dependency again and rewrite kavach.lock"],
  ["kavach test [file.kav]", "run examples, test blocks and generated tests"],
  ["kavach check [file.kav]", "only check for errors; --format json for tools"],
  ["kavach fmt [--check]", "format every file, or report the ones that aren't"],
  ["kavach fix <files or folders>", "apply the fixes that have one right answer and add no error"],
  ["kavach audit", "list unsafe code, revealed secrets and capability use"],
  ["kavach approvals", "list the agents' requests that wait for a person"],
  ["kavach approve <id> --as <name>", "approve a waiting request; kavach deny <id> --as <name> [--reason …] refuses it"],
  ["kavach proto file.kav", "print the .proto file for the program's types"],
  ["kavach outline, query, context", "answer questions about the code, for AI assistants"],
  ["kavach mcp, kavach lsp", "serve the tools to AI assistants, and errors as you type to editors"],
  ["kavach --version", "print the compiler's version and the commit it was built from"],
  ["kavach version <program>", "print what built an executable: its package's name and version, and the compiler's version and commit"],
];

const later = ["kavach repl and kavach eval", "kavach api check", "kavach build --release", "kavach run --watch"];

// ---------- The kavach Command ----------

export const commandDoc: LessonDoc = {
  lead: "One program, kavach, makes, runs, builds, tests and checks your code.",
  simple: <><C>kavach</C> is like a toolbox with one handle: one program holds every tool. You learn one name, then add a word for the job, such as <C>kavach run</C> or <C>kavach test</C>.</>,
  minutes: 4,
  before: ["start/install"],
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Command</th>
              <th className="py-2 font-bold">Does</th>
            </tr>
          </thead>
          <tbody>
            {today.map(([cmd, does]) => (
              <tr key={cmd} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{cmd}</C></td>
                <td className="py-2 text-muted">{does}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>Inside a project, <C>run</C>, <C>build</C>, <C>test</C> and <C>check</C> need no file name: they use <C>src/main.kav</C>.</P>
      <Note>Coming later, as spec 10.5 describes: {later.join(", ")}.</Note>
      <div><SrcRef file="docs/spec/10-modules-and-packages.md" label="spec 10.5 · command line" /></div>
    </>
  ),
};

export const newDoc: LessonDoc = {
  lead: "kavach new my-app makes a project folder you can run at once.",
  simple: <>Like a starter kit, <C>kavach new</C> sets up a folder with everything in place, including a first program that prints a greeting, so you can run it straight away.</>,
  minutes: 1,
  body: (
    <>
      <Terminal commands session={"kavach new my-app\ncd my-app\nkavach run"} />
      <P>
        It writes <C>kavach.toml</C>, a <C>src/main.kav</C> that prints a greeting, and a <C>.gitignore</C> that
        leaves build output out. The name is lowercase, with <C>-</C> between words. What each file is for:{" "}
        <Link href="/docs/tools/project-layout/">Project Layout</Link>.
      </P>
    </>
  ),
};

const run = loadProject("tools/run", ["src/main.kav"]);
const built = loadProject("tools/build", ["src/main.kav"]);
const checked = loadProject("tools/check", ["src/main.kav"]);
const fixed = loadProject("tools/fix", ["src/main.kav"]);
const fixedErrors = loadProject("tools/fix-errors", ["src/main.kav"]);

// What `kavach fix` changes, as spec 12.7 lists it; each row links to the
// page that teaches the mistake.
const fixes: { key: string; mistake: React.ReactNode; writes: React.ReactNode }[] = [
  { key: "dead", mistake: <>An <Link href="/docs/basics/if-else/">else if</Link> or a <Link href="/docs/basics/match/">match arm</Link> that never runs</>, writes: "takes it out" },
  { key: "deprecated", mistake: <>A call to a <Link href="/docs/functions/deprecating-a-function/">deprecated function</Link></>, writes: <>the call its <C>fix</C> says</> },
  { key: "equals", mistake: <Link href="/docs/basics/if-else/"><C>=</C> in a condition</Link>, writes: <C>==</C> },
  { key: "range", mistake: <><Link href="/docs/basics/for-loop/over-a-range/"><C>0..=xs.len()</C></Link> reading <C>xs[i]</C></>, writes: <C>0..xs.len()</C> },
  { key: "odd", mistake: <><C>x % 2 == 1</C>, the odd test that is wrong for negative numbers</>, writes: <C>x % 2 != 0</C> },
  { key: "const", mistake: <>A <C>const</C> that is changed</>, writes: <C>let</C> },
  { key: "uses", mistake: <>A <C>pub</C> function that uses a capability without saying so</>, writes: <>the <C>uses</C> it needs, such as <C>uses Fs.read</C></> },
];
const formatted = loadProject("tools/fmt", ["src/main.kav"]);
// The spec's own example: the commit in it changes with every compiler, so no sample could hold it still.
const versionShown = specBlock("10-modules-and-packages.md", "10.5");
if (!versionShown.includes("kavach version")) throw new Error("spec 10.5: its first code block is no longer the kavach version example");

export const runDoc: LessonDoc = {
  lead: "kavach run checks a program and runs it at once, like a script. Words after -- go to the program.",
  simple: <><C>kavach run</C> is like pressing play: it checks your program and starts it right away, with no extra steps. It is handy while you are trying things out.</>,
  minutes: 2,
  body: (
    <>
      <CodeBlock code={run.files[0].code} file="src/main.kav" />
      <Terminal session={run.output ?? ""} />
      <P>
        <C>kavach run hello.kav</C> runs one file with no project at all. <C>--native</C> compiles it to machine code
        first, and <C>--dry-run</C> runs it without touching the outside world: see{" "}
        <Link href="/docs/agents/agent-modes/dry-runs/">Dry runs</Link>.
      </P>
    </>
  ),
};

export const buildDoc: LessonDoc = {
  lead: "kavach build writes an executable: one file that runs your program at native speed.",
  simple: <><C>kavach build</C> is like baking dough into bread: it turns your program into one finished file that runs the program by itself.</>,
  minutes: 2,
  body: (
    <>
      <Terminal session={built.output ?? ""} />
      <P>
        In a project, the executable goes to <C>.kavach/bin/</C>, named after the project. One file,{" "}
        <C>kavach build hello.kav</C>, becomes <C>hello</C> (<C>hello.exe</C> on Windows) in the current folder;{" "}
        <C>-o</C> names it. See <Link href="/docs/speed/native-executables/">Native Executables</Link>.
      </P>
      <H2>A smaller executable</H2>
      <Terminal commands session={"kavach build --compress hello.kav -o hello\n./hello"} />
      <P>
        <C>--compress</C> packs the executable without losing anything: it unpacks itself when it starts, and the
        build prints what it saved, such as <C>packed: 571 KB, 42% of 1346 KB</C>. <C>--compress=max</C> packs
        harder. Some antivirus tools distrust programs that unpack themselves, which is why packing is asked for,
        not done by default.
      </P>
      <H2>A program&apos;s version</H2>
      <P>
        Every executable carries its package&apos;s <C>name</C> and <C>version</C> from <C>kavach.toml</C>, and the
        version and commit of the compiler that built it. <C>kavach version</C> prints them, so a program found on a
        server can be traced to its source:
      </P>
      <Terminal session={versionShown} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A program built from one file has the file&apos;s name and no version.</li>
        <li>
          With <C>version_flag = true</C> under <C>[build]</C> in <C>kavach.toml</C>, the program answers{" "}
          <C>--version</C> itself: given that one argument, it prints the same two lines and ends with exit code 0,
          before <C>main</C> runs. This holds however it runs: <C>kavach run -- --version</C>, <C>run --native</C> and
          the built executable.
        </li>
        <li>Without the setting, <C>--version</C> is an argument like any other, for the program to read.</li>
        <li>
          A program linked with the run-time library of another Kavach stops at its start with exit code 2 and a
          message saying so, instead of calling the wrong operations.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/10-modules-and-packages.md" label="spec 10.5 · a program's version" />
        <SrcRef file="tools/cli/tests/packed.rs" label="packed executables, tested" />
        <SrcRef file="tools/cli/tests/version.rs" line={26} symbol="a_program_says_what_built_it_and_answers_version_only_when_asked_to" label="its test, in every way of running" />
        <SrcRef file="runtime/native/src/rt_std.rs" line={359} symbol="a_program_for_other_operations_is_stopped_with_a_reason" label="the run-time library's check" />
      </div>
    </>
  ),
};

export const testDoc: LessonDoc = {
  lead: "kavach test runs a file's examples and test blocks, then tests it generates from your types and contracts.",
  simple: <><C>kavach test</C> is like a teacher checking homework: it runs the checks you wrote, then makes up more of its own from the rules your code states, to find what you missed.</>,
  minutes: 1,
  body: (
    <>
      <Terminal commands session={"kavach test orders.kav\nkavach test orders.kav --seed 7"} />
      <P>
        <C>--seed</C> repeats a run exactly. How tests come from types and contracts:{" "}
        <Link href="/docs/errors/testing/">Testing</Link>.
      </P>
    </>
  ),
};

export const checkDoc: LessonDoc = {
  lead: "kavach check finds every error without running anything: quick, and safe to run on any code.",
  simple: <><C>kavach check</C> is like a spell checker for code: it reads your program and lists every mistake without running it, so nothing in the program can happen by accident.</>,
  minutes: 1,
  body: (
    <>
      <Terminal session={checked.output ?? ""} />
      <P>
        A file with mistakes lists each one with its line, as on every page of this course. <C>--format json</C> gives one compact line per message, for editors, scripts and AI tools. Every message
        says what to change; see <Link href="/docs/start/reading-errors/">Reading Errors</Link>.
      </P>
    </>
  ),
};

export const fixDoc: LessonDoc = {
  lead: "kavach fix makes the changes that have one right answer, in every file you name, and says what it changed.",
  simple: <>Like a spell checker that corrects only the mistakes with one sure answer, <C>kavach fix</C> fixes those in your files, tells you each change, and leaves the unsure ones for you to decide.</>,
  minutes: 2,
  before: ["tools/the-kavach-command/check"],
  body: (
    <>
      <CodeBlock code={fixed.files[0].code} file="src/main.kav, before" />
      <Terminal session={fixed.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The run warns twice: an <Link href="/docs/basics/if-else/">else if that can never be true</Link>, and a{" "}
          <Link href="/docs/basics/match/">match arm that can never match</Link>. Each has one right fix: take it out.
        </li>
        <li><C>kavach fix src</C> takes out both and prints one line per fix. Run again, there is <C>nothing to fix</C>, and the program runs without warnings.</li>
        <li>The <C>else</C> after the removed branch stays, joined to the branch before, so <C>grade</C> still has an answer for every score.</li>
      </ul>
      <H2>Errors it fixes too</H2>
      <P>A program that doesn&apos;t compile can be fixed the same way, when each error has one right answer:</P>
      <CodeBlock code={fixedErrors.files[0].code} file="src/main.kav, before" />
      <Terminal session={fixedErrors.output ?? ""} />
      <P>Four messages, four fixes, and the program runs. Everything <C>kavach fix</C> does today:</P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">The mistake</th>
              <th className="py-2 font-bold">What it writes</th>
            </tr>
          </thead>
          <tbody>
            {fixes.map(({ key, mistake, writes }) => (
              <tr key={key} className="border-b border-line align-top">
                <td className="py-2 pr-4">{mistake}</td>
                <td className="py-2 text-muted">{writes}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">It never makes an edit that adds an error.</strong> A fix that wouldn&apos;t compile where it is, such as a replacement that isn&apos;t imported there, is left, with its message.</li>
        <li>A fix that may be wrong is left to you. A misspelled name has a closest match, but it may not be the one you meant, so only the editor offers &quot;Change to <C>total</C>&quot;, and you choose. The same holds for a <C>let</C> that hides another variable.</li>
        <li>The editor offers every fix one at a time, as quick fixes: see <Link href="/docs/tools/editor-setup/">Editor Setup</Link>.</li>
        <li>Which everyday mistakes it fixes, measured: <Link href="/docs/start/mistakes-caught-before-a-run/">Mistakes Caught Before a Run</Link>.</li>
      </ul>
    </>
  ),
};

// ---------- Formatter ----------

export const formatterDoc: LessonDoc = {
  lead: "kavach fmt lays out every file the same way. It has no options, so teams never argue about style.",
  simple: <><C>kavach fmt</C> is like a newspaper&apos;s house style: every file is laid out the same way, with the same spacing, so nobody has to think or argue about it.</>,
  minutes: 3,
  before: ["tools/the-kavach-command"],
  body: (
    <>
      <CodeBlock code={formatted.files[0].code} file="src/main.kav, before" />
      <Terminal session={formatted.output ?? ""} />
      <P>
        After <C>kavach fmt</C>, the body is indented 4 spaces, the two blank lines in a row become one, and{" "}
        <C>totalPrice</C> is renamed <C>total_price</C> everywhere. It sets only the layout of lines, never what is on
        them:
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>4 spaces for each open <C>{"{"}</C>, <C>[</C> or <C>(</C>; several opened on one line count once.</li>
        <li>One more level for a line that continues the one before, such as a line starting with <C>.</C> in a chain.</li>
        <li>A comment line is indented like the code below it.</li>
        <li>No spaces at line ends, at most one blank line in a row, none just inside <C>{"{ … }"}</C>, and one newline at the end of the file.</li>
        <li>Text written over several lines is never changed.</li>
      </ul>
      <P>
        <C>--check</C> changes nothing: it lists the files that aren&apos;t formatted and fails, for a check before
        code is merged. It also fixes names to the language&apos;s style, renaming a declaration and every use, except
        a <C>pub</C> name, which other files may use.
      </P>
      <div><SrcRef file="docs/spec/12-maintainability.md" label="spec 12.7 · one way to write it" /></div>
    </>
  ),
};

// ---------- Editor Setup ----------

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
      <div><SrcRef file="editors/vscode/README.md" label="the extension's README" /></div>
    </>
  ),
};
