import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Build a Command-Line Tool ----------
//
// One task from start to finish: guides/top-words.project, a tool that
// prints a text file's most common words. Its `runs` file is the story
// (check, run, mistakes, test), run by check-samples like every sample;
// the page shows the code a piece at a time, cut from the checked file,
// and each command's output cut from the checked session.

const tool = guideProject("top-words", ["src/main.kav", "kavach.toml", "notes.txt"]);
const piece = (start: string) => tool.piece("src/main.kav", start);
const said = tool.said;

export const guideCliDoc: LessonDoc = {
  lead: "One task from start to finish: a command-line tool that reads a text file and prints its most common words, with options, clear mistakes, tests and a program to ship.",
  simple: <>You will write a small program people run in a terminal, like <C>git</C> or <C>ls</C>: it takes a file and a number, and prints an answer. Along the way the compiler and the tests catch the mistakes.</>,
  minutes: 12,
  before: ["tools/the-kavach-command", "standard-library/files"],
  body: (
    <>
      <P>
        By the end, <C>top-words --input notes.txt --top 2</C> prints the two words the file uses most. Every piece of
        code and every output on this page comes from one project the site builds and runs.
      </P>
      <H2>1. Make the project</H2>
      <P>
        <C>kavach new top-words</C> makes the folder with a <C>kavach.toml</C> and a <C>src/main.kav</C> (see{" "}
        <Link href="/docs/tools/the-kavach-command/new/">kavach new</Link>). The package&apos;s name is the
        program&apos;s name:
      </P>
      <CodeBlock code={tool.file("kavach.toml")} file="kavach.toml" lang="toml" />
      <P>A text file to try it on, in the project&apos;s folder:</P>
      <CodeBlock code={tool.file("notes.txt")} file="notes.txt" lang="text" />
      <H2>2. Say what it takes, as a type</H2>
      <P>
        The command line is a record type. Each field is an option: <C>input</C> is <C>--input</C>, and a field with a
        default may be left out. <C>cli.parse</C> reads the arguments into it, and writes the usage for you:
      </P>
      <CodeBlock code={piece("type Args")} file="src/main.kav" />
      <H2>3. Write the work as small functions</H2>
      <P>
        Counting is a function of its own, with an <C>example</C> that says what it gives. Anything that isn&apos;t a
        letter, such as a comma, separates words:
      </P>
      <CodeBlock code={piece("fn count_words")} file="src/main.kav" />
      <P>
        Picking the most common is another. Its <C>require</C> says what a caller must give, and the compiler and the
        tests hold every caller to it:
      </P>
      <CodeBlock code={piece("fn most_common")} file="src/main.kav" />
      <H2>4. Join them in main, with clear mistakes</H2>
      <P>
        <C>main</C> parses the options, reads the file and prints. Each way it can go wrong says what happened and ends
        the program with a code: 2 for a wrong command line, 1 for a file it can&apos;t read, as most tools do:
      </P>
      <CodeBlock code={piece("fn main")} file="src/main.kav" />
      <H2>5. Check it, then run it</H2>
      <P>
        <C>kavach check</C> compiles without running: types, capabilities and contracts. Then run it with options, as
        a user would:
      </P>
      <Terminal session={said("check")} />
      <Terminal session={said("run src/main.kav --input notes.txt")} />
      <Terminal session={said("run src/main.kav --input notes.txt --top 2")} />
      <H2>6. What a user sees when they get it wrong</H2>
      <P><C>--help</C> gives the usage, written from the type:</P>
      <Terminal session={said("run src/main.kav --help")} />
      <P>A required option left out says which, then the usage:</P>
      <Terminal session={said("run src/main.kav --top 2")} />
      <P>A file that isn&apos;t there is an error value, not a crash:</P>
      <Terminal session={said("run src/main.kav --input missing.txt")} />
      <Note>
        The usage says <C>main</C> because the program is run from <C>src/main.kav</C>. <C>cli.exit</C> sets the
        code a script or CI job sees; the terminal doesn&apos;t show it.
      </Note>
      <H2>7. Test it</H2>
      <P>
        <C>kavach test</C> runs each <C>example</C>, then calls every function with generated values: 200 cases each,
        always meeting its <C>require</C>. No test file was written. <C>--seed 7</C> makes the run the same each time:
      </P>
      <Terminal session={said("test --seed 7")} />
      <P>
        A case that fails is shrunk to the smallest one and saved, so it is checked on every run after. See{" "}
        <Link href="/docs/errors/contracts/">Contracts</Link> and{" "}
        <Link href="/docs/tools/the-kavach-command/test/">kavach test</Link>.
      </P>
      <H2>8. Build it and ship it</H2>
      <P>
        <C>kavach build</C> compiles native code and writes the program to <C>.kavach/bin/top-words</C>, or{" "}
        <C>top-words.exe</C> on Windows. Its runtime is inside it, so the machine it runs on needs no Kavach. Copy it
        anywhere on the PATH, and <C>top-words --input notes.txt</C> works like any other command. See{" "}
        <Link href="/docs/tools/the-kavach-command/build/">kavach build</Link>.
      </P>
      <H2>The whole program</H2>
      <CodeBlock code={tool.file("src/main.kav")} file="src/main.kav" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/06-io-and-cli.md" label="spec 14.6 · std.cli" />
        <SrcRef file="docs/spec/10-modules-and-packages/03-project-layout.md" label="spec 10.3 · where the program goes" />
        <SrcRef file="docs/spec/11-testing.md" label="spec 11 · testing" />
        <SrcRef file="website/content/samples/guides/top-words.project/runs" label="the commands this page runs" />
      </div>
    </>
  ),
};
