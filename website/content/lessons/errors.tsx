import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";
import { Table } from "./spec-table";

// How a program ends, as spec 5.8 lists it.
const stops = specTables("05-errors.md", "5.8")[0];

const panicInTask = loadProject("tasks/panic-in-task", ["src/main.kav"]);

// ---------- Errors ----------

export const errorsDoc: LessonDoc = {
  lead: "A function that can fail says so with ! after its result type, and every caller must deal with it. There are no exceptions.",
  simple: <>Some jobs can fail, like a delivery when nobody is home. In Kavach, a function that can fail says so on its label, and whoever calls it must plan for that, so a failure never comes as a surprise.</>,
  minutes: 6,
  before: ["functions/functions", "types/unions"],
  body: (
    <>
      <H2>Syntax</H2>
      <Syntax code={"fn name(…) -> Result ! ErrorType {\n    fail ErrorType.SomeError    // stop and give back an error\n}"} />
      <H2>Example</H2>
      <Ex name="errors/declare" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>! AgeError</C> says <C>parse_age</C> gives an <C>Int</C>, or fails with an <C>AgeError</C>.</li>
        <li><C>fail AgeError.Empty</C> stops the function and gives back that error. A variant is always written with its union&apos;s name.</li>
        <li>An error type is an ordinary union: each variant is one way to fail, and can carry details.</li>
      </ul>
      <P>The caller can&apos;t forget. Using the result as if nothing could go wrong is refused:</P>
      <Ex name="errors/unhandled.fails" />
      <Note>
        Three ways to deal with an error, each on its own page: pass it on with <Link href="/docs/errors/try/">try</Link>,
        handle it with <Link href="/docs/errors/catch/">catch</Link>, or treat each case with{" "}
        <Link href="/docs/errors/matching-errors/">match</Link>.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Write <C>fn half(n: Int) -&gt; Int ! String</C> that fails with a message for odd numbers.</>,
    answer: <Ex name="safety/sub-default" />,
  },
  deeper: (
    <>
      <P>
        Exceptions make failure invisible where a function is called: any line might throw, so every line is
        suspect, and the real flow of the program lives in your head. Kavach puts failure in the type: a call that
        can fail doesn&apos;t compile until the code says what happens then.
      </P>
      <P>
        Panics still exist, but only for bugs, such as a broken index or an overflow, and they stop the program at
        an exact line instead of letting it carry on wrong. An error travels back as a second return value that the
        compiler passes along for you, not through stack-unwinding machinery.
      </P>
    </>
  ),
};

// ---------- try ----------

export const tryDoc: LessonDoc = {
  lead: "try passes an error on to your own caller: if the call fails, your function fails with the same error.",
  simple: <><C>try</C> is like passing a problem up to your manager: if this step fails, you stop and hand the same problem to whoever asked you to do the job.</>,
  minutes: 4,
  before: ["errors/errors"],
  body: (
    <>
      <Ex name="errors/try" />
      <P>
        <C>total_age</C> calls <C>parse_age</C> twice. Each <C>try</C> means: if this fails, stop here and give the
        error to my caller. The happy path reads straight down, with no checks in between.
      </P>
      <Note>
        <C>try</C> only works in a function that can fail with that error itself: here, both are{" "}
        <C>! AgeError</C>.
      </Note>
      <H2>Methods that take a function that can fail</H2>
      <P>
        A built-in method that takes a function (<C>map</C>, <C>filter</C>, <C>sum</C>, <C>sort_by</C>,{" "}
        <C>Shared.update</C>, …) passes that function&apos;s error on. Given a function that can fail, the call can
        fail with its error, and the method stops at the first one:
      </P>
      <Ex name="errors/methods-pass-on" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The <C>map</C> call is the one that can fail, so it is the one marked, with <C>try</C> or <C>catch</C>.</li>
        <li>A lambda that handles the error inside, as in the <C>sum</C>, can&apos;t fail, and neither can the call.</li>
        <li><C>Shared.update</C> given a function that fails leaves the shared value as it was.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/05-errors.md" label="spec 5.2 · try" />
        <SrcRef file="tests/conformance/05-errors/built-in-methods-pass-a-functions-error-on.kav" label="tested" />
      </div>
    </>
  ),
};

// ---------- catch ----------

export const catchDoc: LessonDoc = {
  lead: "catch handles an error on the spot: give a value to use instead, or run code with the error.",
  simple: <><C>catch</C> deals with a problem right where it happens, like using a spare key when you lose yours: use a value to fall back on, or do something about the error.</>,
  minutes: 4,
  before: ["errors/errors"],
  body: (
    <>
      <H2>A value to use instead</H2>
      <Ex name="safety/sub-default" />
      <H2>Code that sees the error</H2>
      <Ex name="safety/sub-handle" />
      <H2>Skip and carry on</H2>
      <P>A handler can also leave: <C>continue</C> skips to the next round of a loop, <C>return</C> leaves the function.</P>
      <Ex name="errors/skip" />
    </>
  ),
};

// ---------- Matching Errors ----------

export const matchErrorsDoc: LessonDoc = {
  lead: "match on a call that can fail: ok(value) for success, err(…) for each kind of error.",
  simple: <>Like a help desk with a different answer for each kind of problem: one reply when all went well, and one for each way it could go wrong. Kavach checks that no kind is left without an answer.</>,
  minutes: 4,
  before: ["errors/errors", "basics/match"],
  body: (
    <>
      <Ex name="errors/match" />
      <P>
        Each error variant gets its own message, with its details taken apart. As with every <C>match</C>, leaving a
        case out is a compile error, so a new kind of error can&apos;t slip through unhandled.
      </P>
    </>
  ),
};

// ---------- The Error Catch-all ----------

export const errorCatchAllDoc: LessonDoc = {
  lead: "! Error accepts any error. Use it in scripts and in main, where you just want to stop with a message.",
  simple: <><C>! Error</C> is like one bin for every kind of problem. It suits small scripts, where you just want to stop and say what went wrong. A library should name its errors, so callers know what to expect.</>,
  minutes: 3,
  before: ["errors/try"],
  body: (
    <>
      <Ex name="errors/main-error" />
      <P>
        If any <C>try</C> in <C>main</C> fails, the program prints <C>error:</C> and the message to standard error,
        and exits with code 1. <C>Error(&quot;…&quot;)</C> makes an error that is only a message:
      </P>
      <Ex name="safety/sub-message" />
      <Note>
        A caller can still ask which error an <C>Error</C> holds, with <C>match e {"{"} FsError.NotFound(p) =&gt; …, _ =&gt; … {"}"}</C>:
        see <Link href="/docs/errors/kinds-of-errors/">Kinds of Errors</Link>. In a library, name your errors
        (<C>! AgeError</C>), so callers know exactly what can go wrong.
      </Note>
    </>
  ),
};

// ---------- Adding Context ----------

export const contextDoc: LessonDoc = {
  lead: "context adds a short note to an error that try passes on, so the message says what was being done.",
  simple: <>Like adding a note to a problem as you pass it on: &quot;this happened while loading settings&quot;. The original problem stays inside, and the message now also says what you were doing.</>,
  minutes: 3,
  before: ["errors/try"],
  body: (
    <>
      <Ex name="errors/context" />
      <P>
        The message becomes <C>loading settings: not a number: 80a</C>. The original error is kept inside, so a
        caller can still <C>match</C> on it, whatever context was added:
      </P>
      <Ex name="errors/match-error-context" />
    </>
  ),
};

// ---------- Panics ----------

export const panicsDoc: LessonDoc = {
  lead: "A panic stops the program at once, or only its task in a task. It is for bugs, not for failures you expect.",
  simple: <>A panic is like the emergency stop button on a machine: it halts the work at once. It is meant for mistakes in the program itself, not for everyday problems like a missing file.</>,
  minutes: 4,
  before: ["errors/errors"],
  body: (
    <>
      <Ex name="errors/share.panics" />
      <P>A panic happens on:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>an index past the end of a list, and a division by zero;</li>
        <li>a whole number too big for its type (checked in every build);</li>
        <li>a broken <Link href="/docs/errors/contracts/">contract</Link>, and calls nested too deeply;</li>
        <li>running out of memory: <C>panic: out of memory (asked for N bytes)</C>, with no file and line, because nothing more may be asked of memory;</li>
        <li>an explicit <C>panic(&quot;…&quot;)</C>.</li>
      </ul>
      <Note>
        Expected failures, such as bad input or a missing file, are errors (<C>!</C>), not panics. A panic means the
        program itself is wrong.
      </Note>
      <H2>A panic in a task</H2>
      <P>
        A panic in a <Link href="/docs/concurrency/tasks/">task</Link> stops only that task, as in Tokio and Java.
        The rest of the program goes on:
      </P>
      <CodeBlock code={panicInTask.files[0].code} file="src/main.kav" />
      <Terminal session={panicInTask.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          The panic is printed to standard error when it happens, with its place. It is shown after the program&apos;s
          output here, but it came before <C>panicked: …</C>.
        </li>
        <li>
          <C>.wait()</C> on that task fails with an error whose message is{" "}
          <C>panicked: &lt;message&gt; at &lt;file&gt;:&lt;line&gt;</C>.
        </li>
        <li>The program ends as usual, with code <C>0</C>.</li>
        <li>A panic in the main task still ends the program, with code <C>101</C>.</li>
        <li>
          A panic in an agent&apos;s handler stops just that handler, and the send fails the same way: see{" "}
          <Link href="/docs/agents/running-an-agent/">Running an Agent</Link>.
        </li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0065-a-panic-stops-only-its-task.md" label="decision 0065 · a panic stops only its task" />
        <SrcRef file="tests/conformance/09-concurrency/a-panic-stops-only-its-task.kav" label="a panic in a task, tested" />
      </div>
    </>
  ),
};

export const panicsWhatDoc: LessonDoc = {
  lead: "Write panic(\"message\") for a state that must never happen. For overflow you expect, use checked_add.",
  simple: <>Use <C>panic</C> as an alarm for something that should be impossible. When a number might simply grow too big, <C>checked_add</C> lets you handle that calmly instead.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="errors/explicit.panics" />
      <H2>When overflow is possible</H2>
      <P>
        <C>checked_add</C> gives <C>none</C> instead of panicking, and <C>wrapping_add</C> wraps around on purpose,
        where that is what you mean:
      </P>
      <Ex name="errors/checked" />
    </>
  ),
};

export const stopsDoc: LessonDoc = {
  lead: "How a program ends tells the shell what happened: 0 for success, 1 for an error from main, 101 for a panic in main.",
  simple: <>When a program ends, it leaves a short note for whoever started it, like a receipt: one number that says whether it finished well, failed with an error, or panicked.</>,
  minutes: 2,
  body: (
    <>
      <Table header={stops.header} rows={[["It finished", "nothing", "`0`"], ...stops.rows]} wrapFirst />
      <P>
        These messages never go to standard output, so a program&apos;s real output stays clean when it is piped
        into another program.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          Ctrl+C, a closed terminal and <C>SIGTERM</C> end the program the way <C>cli.exit</C> does: what it printed
          is flushed, and logs and traces are sent before it ends. A second Ctrl+C ends it at once.
        </li>
        <li>This holds for <C>kavach run</C> and for programs built with <C>kavach build</C>.</li>
      </ul>
      <SrcRef file="docs/decisions/0062-crashes-restarts-and-clocks.md" label="decision 0062 · crashes and clocks" />
    </>
  ),
};
