import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";

const query = loadProject("errors/query", ["src/main.kav"]);

// ---------- Kinds of Errors ----------

const kinds: [string, string, string, string][] = [
  ["Compile error", "before the program runs", "No: fix it first", "a misspelt name"],
  ["Warning", "before the program runs", "Yes", "a variable nobody uses"],
  ["Error (a failure value)", "while it runs, and expected", "Yes, the code handles it", "text that isn't a number"],
  ["Panic", "while it runs: a bug", "No: it stops at once", "dividing by zero"],
];

const stdErrors: [string, string, string][] = [
  ["ParseError", "Reading text as a number, Bool or Char: Int(text)", "02-types/01-built-in-types.md"],
  ["FsError", "Files: NotFound(path), Denied(path), Failed(path, reason)", "14-standard-library/03-fs.md"],
  ["NetError", "Connections: Refused, TimedOut, Failed", "14-standard-library/10-net.md"],
  ["HttpError", "HTTP: the network, a bad URL, a bad message, an error status, or JSON that doesn't fit", "14-standard-library/11-http.md"],
  ["JsonError", "JSON: bad syntax (with line and column), too deep, too large, a repeated key, or a field that doesn't match", "02-types/12-json.md"],
  ["GrpcError", "gRPC calls", "14-standard-library/14-grpc.md"],
  ["RegexError", "A pattern that isn't valid", "14-standard-library/08-regex.md"],
  ["CliError", "Command-line arguments that don't fit", "14-standard-library/06-io-and-cli.md"],
  ["AiError", "Asking an AI model: no key set, the provider refused, an answer of the wrong shape, or a limit said no", "08-capabilities-and-agents/04-typed-ai-calls.md"],
  ["Error", "Any error at all: the catch-all for scripts and main", "05-errors.md"],
];

// scripts/check-refs.mjs reads only literal SrcRef paths, so these are checked here, at build time.
for (const [name, , file] of stdErrors) {
  if (!fs.existsSync(path.join(process.cwd(), "..", "docs", "spec", file))) {
    throw new Error(`Kinds of Errors: the spec file for ${name}, ${file}, no longer exists`);
  }
}

export const errorKindsDoc: LessonDoc = {
  lead: "Kavach reports problems in four ways: compile errors and warnings before the program runs, errors and panics while it runs.",
  simple: <>Think of checking a recipe before you cook: some problems are found while reading it, before anything starts. Others only show up while cooking: normal ones you planned for, like running out of milk, or real mistakes that stop everything.</>,
  minutes: 6,
  before: ["start/reading-errors"],
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Kind</th>
              <th className="py-2 pr-4 font-bold">When</th>
              <th className="py-2 pr-4 font-bold">Does the program run?</th>
              <th className="py-2 font-bold">For example</th>
            </tr>
          </thead>
          <tbody>
            {kinds.map(([kind, when, runs, ex]) => (
              <tr key={kind} className="border-b border-line align-top">
                <td className="py-2 pr-4 font-bold text-ink">{kind}</td>
                <td className="py-2 pr-4 text-muted">{when}</td>
                <td className="py-2 pr-4 text-muted">{runs}</td>
                <td className="py-2 text-muted">{ex}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <H2>1. Compile errors</H2>
      <P>The compiler refuses the program, says what is wrong, and usually what to write instead. Nothing runs.</P>
      <Ex name="errors-intro/typo.fails" />

      <H2>2. Warnings</H2>
      <P>The program runs, but the compiler points at something that is probably a mistake, with the fix:</P>
      <Ex name="errors/unused.warns" />
      <Ex name="errors/float-eq.warns" />
      <Ex name="errors/odd.warns" />
      <Ex name="errors/catch-nothing.warns" />

      <H2>3. Errors: failures you expect</H2>
      <P>
        Bad input, a missing file, a network that is down: these are normal, so they are values the program handles.
        A function that can fail says so with <C>!</C>, and the caller must deal with it (the{" "}
        <Link href="/docs/errors/errors/">Errors</Link> page shows how):
      </P>
      <Ex name="errors/parse" />
      <Ex name="errors/file" />

      <H2>4. Panics: bugs</H2>
      <P>
        A panic means the program itself is wrong: an index past the end, a division by zero, a broken contract. It
        stops at once instead of carrying on with a wrong value:
      </P>
      <Ex name="errors/share.panics" />
      <P>When the compiler already knows the values, it doesn&apos;t wait for the program to run: arithmetic that would always panic is a compile error.</P>
      <Ex name="errors/divzero.fails" />
      <P>The compiler follows a <C>let</C> too, while nothing in between may change it:</P>
      <Ex name="errors/let-known.fails" />
      <P>
        A <C>let</C> is known no more once a branch, a loop or a lambda may set it, once an <C>if</C>, <C>while</C> or{" "}
        <C>match</C> tests it, or once it is passed to an <C>inout</C> parameter. The test may be what keeps the
        arithmetic safe, so this one compiles and runs:
      </P>
      <Ex name="errors/let-tested" />
      <P>
        A <C>for</C> over a range of known numbers counts too, when nothing in its body can leave early. Each statement
        that always runs is checked for the loop&apos;s first and last values, and the message is that of the first
        round that panics:
      </P>
      <Ex name="errors/loop-known.fails" />
      <P>A <C>+=</C> or <C>-=</C> of a known number is checked for all its rounds:</P>
      <Ex name="errors/loop-adds.fails" />
      <P>A known whole number converted to a whole-number type it fits stays known, so this is caught too:</P>
      <Ex name="errors/loop-convert.fails" />
      <P>
        A <C>break</C>, <C>continue</C>, <C>return</C>, <C>fail</C> or <C>try</C> in the body may be what keeps that
        round from running, so the loop is not checked. Nor is a statement inside an <C>if</C>, a <C>match</C> or an
        inner loop that may not run:
      </P>
      <Ex name="errors/loop-break" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/01-basics.md" label="spec 1.8 · the rule" />
        <SrcRef file="tests/conformance/01-basics/a-known-loop-that-always-panics-is-an-error.kav" label="past the end, tested" />
        <SrcRef file="tests/conformance/01-basics/a-known-loop-that-adds-past-its-type-is-an-error.kav" label="adds past its type, tested" />
        <SrcRef file="tests/conformance/01-basics/a-known-value-through-a-conversion-is-checked.kav" label="through a conversion, tested" />
        <SrcRef file="tests/conformance/01-basics/a-loop-that-may-not-reach-a-round-is-not-assumed.kav" label="loops not checked, tested" />
      </div>
      <Note>
        The rule of thumb: if it can happen in a correct program (the user typed rubbish), it is an error. If it can
        only happen in a wrong program, it is a panic.
      </Note>

      <H2>Error types from the standard library</H2>
      <P>Each module that can fail has its own error type, a union you can <C>match</C> on:</P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            {stdErrors.map(([name, what, file]) => (
              <tr key={name} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{name}</C></td>
                <td className="py-2 pr-4 text-muted">{what}</td>
                <td className="py-2"><SrcRef file={`docs/spec/${file}`} label="spec" /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Calls to a capability, such as <C>Payments.refund</C>, have one more case. Any of them can be refused, by a
        limit, by a person who says no, or because nothing implements the capability, so every capability&apos;s
        error type has the case <C>Denied(reason: String)</C>, and the compiler insists on it. See{" "}
        <Link href="/docs/agents/your-own-capabilities/when-a-call-is-refused/">When a call is refused</Link>.
      </P>

      <H2>Which error can it be?</H2>
      <P>
        Other languages make you guess which exceptions a call can throw, or test the type of the error at run time.
        In Kavach the first line of a function names its error type, so you know before you call it:
      </P>
      <Ex name="errors/which" />
      <P>
        <C>! FsError</C> says <C>load</C> can only fail with a file error, and <C>match</C> handles each case. Leave
        one out, and the compiler names it:
      </P>
      <Ex name="errors/which-missed.fails" />
      <P>To find every function that can fail with an error type, directly or passed on through <C>! Error</C>, ask the compiler:</P>
      <Terminal session={query.output ?? ""} />
      <H2>Which error is inside an Error?</H2>
      <P>
        <C>! Error</C> accepts any error. A caller can still ask which one it holds, as Python&apos;s{" "}
        <C>except FileNotFoundError</C> or Go&apos;s <C>errors.As</C> do: name the type with each case.
      </P>
      <Ex name="errors/match-error" />
      <P>
        An <C>Error</C> can hold any error type, so each case names its type (<C>FsError.NotFound</C>), and a{" "}
        <C>_</C> arm handles the rest; here it passes the error on with <C>fail e</C>. Both are checked:
      </P>
      <Ex name="errors/match-error-bare.fails" />
      <Ex name="errors/match-error-no-default.fails" />
      <Note>
        In a library, still prefer a specific error type such as <C>! FsError</C>: the compiler then checks that every
        case is handled, with no <C>_</C> needed.
      </Note>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>A user types &quot;abc&quot; where your program asked for a number. What should that be?</>,
      options: ["An error the program handles", "A panic", "A compile error"],
      answer: 0,
      explain: "Bad input happens in correct programs, so it is an error value: Int(text) can fail, and the code says what to do.",
    },
  ],
};
