import Link from "next/link";
import { Terminal } from "@/components/terminal";
import { C } from "@/components/inline-code";
import versions from "@/data/versions.json";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- What a Program Is ----------

export const programDoc: LessonDoc = {
  lead: "A program is a list of steps for the computer, written in a language both you and the computer can read.",
  simple: <>A program is like a recipe: steps written in order, which the computer follows exactly, very fast, and nothing more. Before anything runs, Kavach reads the whole recipe and tells you about any mistake it finds.</>,
  minutes: 4,
  body: (
    <>
      <P>
        A computer does exactly what it is told, very fast, and nothing else. A <strong className="text-ink">program</strong>{" "}
        is the telling: steps written down in order. This is a whole Kavach program:
      </P>
      <Ex name="start/script" />
      <P>
        Each line is one step. The first prints a greeting. The second gives the number 2026 a name,{" "}
        <C>year</C>. The third prints a sentence with that number inside it. The box under the code shows what the
        program printed when it ran.
      </P>
      <H2>From text to running program</H2>
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li>You write the steps in a text file, the <strong className="text-ink">source code</strong>. Kavach files end in <C>.kav</C>, and are saved as UTF-8 text, as most editors save by default. The invisible mark some Windows editors put at the start of a file (a byte order mark) is fine: Kavach reads it as a space.</li>
        <li>The <strong className="text-ink">compiler</strong> reads the file and checks it. If something is wrong, it tells you what, and where.</li>
        <li>If all is well, it turns the file into instructions the processor runs directly.</li>
      </ol>
      <Note>
        You don&apos;t need to know how a processor works. You write the steps; Kavach checks them and makes them run.
      </Note>
    </>
  ),
  quiz: [
    {
      type: "bool",
      prompt: <>The compiler runs your program even when it finds a mistake, and the mistake shows up later.</>,
      answer: false,
      explain: "Kavach checks the whole file first. A mistake it can find is reported before anything runs.",
    },
  ],
};

// ---------- The Terminal ----------

export const terminalDoc: LessonDoc = {
  lead: "The terminal is a window where you type commands. You need four of them to run Kavach programs.",
  simple: <>The terminal is a way to talk to your computer by typing instead of clicking. You type a short command, press Enter, and the computer does it, like giving a helper instructions one sentence at a time.</>,
  minutes: 5,
  body: (
    <>
      <P>
        On Windows, open <strong className="text-ink">Terminal</strong> or <strong className="text-ink">PowerShell</strong>{" "}
        from the Start menu. On macOS, open <strong className="text-ink">Terminal</strong>; on Linux, your terminal app.
        A line with a blinking cursor appears: type a command there and press Enter.
      </P>
      <H2>The commands you need</H2>
      <Terminal
        commands
        session={"cd projects # go into the folder named projects\ncd .. # go back up one folder\nls # list the files here (dir works too on Windows)\nkavach run hello.kav # check hello.kav, then run it"}
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>cd</C> moves you into a folder: run your program from the folder its file is in.</li>
        <li>Press the Up arrow to bring back the last command, instead of typing it again.</li>
        <li>Press Ctrl+C to stop a program that keeps running.</li>
      </ul>
      <Note>
        An editor such as VS Code has a terminal built in (View › Terminal), already in your project&apos;s folder.
      </Note>
    </>
  ),
};

// ---------- Install ----------

export const installDoc: LessonDoc = {
  lead: "One clone, one build. A single-command installer comes with the first public release.",
  simple: <>Installing puts Kavach on your computer so you can use it. For now you download its source code and build it yourself, a bit like putting together a flat-pack table. It takes a few minutes, and you do it once.</>,
  minutes: 5,
  before: ["start/the-terminal"],
  body: (
    <>
      <P>
        Kavach is young and moves fast, so today you build it from source. You need{" "}
        <a href="https://rustup.rs">Rust</a> (Kavach&apos;s compiler is written in it) and git. Windows, Linux and WSL
        all work.
      </P>
      <Terminal commands session={`git clone ${versions.repo}\ncd kavach\ncargo build --release -p kavach-cli`} />
      <P>
        The build takes a few minutes, once. The program lands at <C>target/release/kavach</C>: add that folder to
        your PATH, and <C>kavach</C> works in every terminal.
      </P>
      <H2>Your editor</H2>
      <P>
        The VS Code extension in <C>editors/vscode</C> adds colors, errors as you type, and go-to-definition.
      </P>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>You wrote hello.kav. Which command runs it?</>,
      options: ["kavach run hello.kav", "kavach hello.kav --exec", "kavach compile hello.kav"],
      answer: 0,
      explain: "run checks the file and runs it at once; build makes an executable instead.",
    },
  ],
};

// ---------- Reading Errors ----------

export const errorsIntroDoc: LessonDoc = {
  lead: "Error messages are help: each one says what is wrong, where, and usually what to write instead.",
  simple: <>An error message is like a note from a careful proofreader: it points to the exact spot, says what is wrong, and often suggests the fix. Read it as help, not as a complaint.</>,
  minutes: 5,
  before: ["start/your-first-program"],
  body: (
    <>
      <P>Everyone writes mistakes, every day. Kavach finds most of them before the program runs:</P>
      <Ex name="errors-intro/typo.fails" />
      <P>
        In the terminal, the message goes on: a line <C>--&gt; typo.kav:3:11</C> names the file, the line and the
        column, and the line itself is shown with <C>^^^^</C> under the mistake. Read the first line first: it is
        usually the fix.
      </P>
      <H2>Types that don&apos;t fit</H2>
      <Ex name="errors-intro/types.fails" />
      <P>
        <C>age + 1</C> is a number, but <C>next</C> was declared as text. Decide which you meant: a number, or{" "}
        <C>String(age + 1)</C>.
      </P>
      <H2>Panics: stops while running</H2>
      <P>
        A few mistakes can only show while the program runs, such as a loop that counts one past the end of a list
        when the count comes from somewhere else. The program stops at once, with the reason, instead of carrying on
        with a wrong value:
      </P>
      <Ex name="errors-intro/panic.panics" />
      <P>
        When the compiler knows both the list and the range, the same loop is a compile error instead: see{" "}
        <Link href="/docs/errors/kinds-of-errors/">Kinds of Errors</Link>.
      </P>
      <Note>
        When a message isn&apos;t clear, that is a bug in Kavach: <Link href={`${versions.repo}/issues`}>tell us</Link>.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Fix the first program above so it prints 42.</>,
    answer: <P>Spell the name as it was declared: <C>print(total + 2)</C>.</P>,
  },
};
