import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject, loadProjectFiles } from "@/lib/samples";
import { Ex, Files, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

const beside = loadProjectFiles("testing/beside");
const missed = loadProjectFiles("testing/missed");
const kept = loadProject("testing/keep", ["stats.kav", "tests/found/stats.kavf"]);

// ---------- Contracts ----------

export const contractsDoc: LessonDoc = {
  lead: "A contract states in one line what a function needs and what it promises, and Kavach checks it.",
  simple: <>A contract is like the terms on a job order: what the customer must provide, and what the worker promises to deliver. Kavach checks that both sides keep their word.</>,
  minutes: 5,
  before: ["functions/signatures", "errors/panics"],
  body: (
    <>
      <Ex name="contracts/ensure" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>require</C>: what must be true when the function is called. If not, the caller has a bug.</li>
        <li><C>ensure</C>: what the function promises about its <C>result</C>.</li>
        <li><C>invariant</C>: what is always true of a type&apos;s values.</li>
        <li><C>example</C>: one real call and its answer.</li>
      </ul>
      <Note>
        When someone else, or an AI, writes the body, you review the contract: it is short, and Kavach checks that
        the body keeps it.
      </Note>
    </>
  ),
};

export const requireDoc: LessonDoc = {
  lead: "require states what callers must make true. A call that breaks it stops the program, naming the rule.",
  simple: <><C>require</C> is like the sign at a fair ride: &quot;you must be this tall&quot;. If a caller doesn&apos;t meet it, the program stops and names the rule, and sometimes Kavach spots it before the program even runs.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="contracts/require.panics" />
      <P>In the terminal, the message also shows the function, the line and the values involved, so the caller&apos;s bug is easy to find.</P>
      <H2>Found before it runs</H2>
      <P>
        When the compiler knows what a call passes (a literal, a <C>const</C> set to one, or a <C>let</C> set to one
        with nothing in between that may change it), and the <C>require</C> compares a parameter with a number, a call that would always break it is a compile error, with the contract
        and the value:
      </P>
      <Ex name="contracts/require-known.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>half(10)</C> keeps the rule, so only the second call is refused.</li>
        <li>Decimals count too: <C>speed(120.0, -2.0)</C> against <C>require hours &gt; 0.0</C> is refused the same way. Whole numbers compare exactly; anything with a decimal compares as <C>Float</C>.</li>
        <li>A <C>require</C> with parts joined by <C>and</C> has each part checked, and the message names every value involved.</li>
        <li>Standard-library contracts count too: <C>random.int(5, 5)</C> is refused the same way.</li>
        <li>A value the compiler can&apos;t know, read from input or worked out in a loop as in <C>speed</C> above, is still checked when the program runs.</li>
      </ul>
      <P>
        A <C>for</C> over a range of known numbers passes its first and last values, when nothing in its body can leave
        early. The message names the first value that breaks the rule:
      </P>
      <Ex name="contracts/require-loop.fails" />
    </>
  ),
};

export const ensureDoc: LessonDoc = {
  lead: "ensure states what the function promises; result is the value it gives back.",
  simple: <><C>ensure</C> is like a guarantee card that comes with a product: it says what will be true of the answer the function gives back, and Kavach checks that the promise is kept.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="contracts/ensure" />
      <P>
        <C>account.balance</C> in the <C>ensure</C> is the balance before the call: parameters are values and never
        change, so no special &quot;old value&quot; syntax is needed. An <C>ensure</C> is checked only when the
        function succeeds.
      </P>
      <H2>Found before it runs</H2>
      <P>
        When the function gives back a value the compiler knows, and an <C>ensure</C> that compares <C>result</C> with a
        number is false for it, that is a compile error. This holds for the end of the function, a branch at its end,
        or a <C>return</C>, even in a branch that may not run: whenever it runs, it panics.
      </P>
      <Ex name="contracts/ensure-known.fails" />
    </>
  ),
};

export const invariantDoc: LessonDoc = {
  lead: "invariant states what is always true of a type. A value that breaks it can't exist.",
  simple: <>An invariant is a rule that is always true for a kind of thing, like &quot;a bank balance never goes below zero&quot;. Kavach checks it every time such a value is made or changed.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="contracts/invariant.panics" />
      <P>The invariant is checked every time an <C>Account</C> is made or changed: the first change is fine, the second would make the balance negative.</P>
    </>
  ),
};

export const exampleDoc: LessonDoc = {
  lead: "example shows one real call and its expected result. kavach test runs every one.",
  simple: <>An <C>example</C> is like a worked example in a school book: one real question with its answer. But this book checks itself: if the answer stops being true, the tests fail.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="testing/sub-contracts.test" />
      <P>
        An example is documentation that can&apos;t go out of date: editors show it on hover, and a wrong one fails
        the tests. Examples are left out of the program you ship.
      </P>
    </>
  ),
};

export const checkedDoc: LessonDoc = {
  lead: "While you develop, every contract is checked. In a release build, public functions' contracts and every invariant are.",
  simple: <>While you are building, Kavach checks every contract, like a teacher checking every step of homework. In the finished program, it keeps the checks on functions marked <C>pub</C>, and on every always-true rule.</>,
  minutes: 3,
  body: (
    <>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            <tr className="border-b border-line"><td className="py-2 pr-4"><C>kavach run</C>, <C>kavach test</C>, debug builds</td><td className="py-2">every contract</td></tr>
            <tr className="border-b border-line"><td className="py-2 pr-4">release builds</td><td className="py-2">contracts of <C>pub</C> functions, and every invariant</td></tr>
          </tbody>
        </table>
      </div>
      <P>A contract must only read values: it can&apos;t change anything or use a capability, so checking it never changes what the program does.</P>
      <div><SrcRef file="docs/spec/07-contracts.md" label="spec 7.4 · release settings" /></div>
    </>
  ),
};

// ---------- Testing ----------

export const testingDoc: LessonDoc = {
  lead: "kavach test tests every function from its types and contracts. Most code needs no test file at all.",
  simple: <>Testing means trying your code to find its mistakes. Kavach does much of this for you: from what your functions say they need and promise, it makes up many tries on its own.</>,
  minutes: 5,
  before: ["errors/contracts"],
  body: (
    <>
      <P>This file has no test in it. <C>kavach test</C> still finds its bug:</P>
      <Ex name="testing/sub-stats.test" />
      <P>
        It tried many lists, found the smallest one that crashes, <C>[]</C>, and suggests three fixes, each with
        when to choose it. The failing case is saved and replayed on every later run.
      </P>
      <H2>A found case becomes an example</H2>
      <P>
        Found cases are saved in <C>tests/found/</C>, one file per module, each line one call. The files are
        generated: commit them, but never edit them. Once the bug is fixed, <C>kavach test --keep</C> turns each
        found case that now passes into an <C>example</C> on its function, so it becomes documentation:
      </P>
      <CodeBlock code={kept.files[0].code} file="stats.kav, after the fix" />
      <CodeBlock code={kept.files[1].code} file={kept.files[1].path} lang="text" />
      <Terminal session={kept.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>--keep</C> wrote <C>example average([]) == 0</C> into <C>stats.kav</C>, and the case left <C>tests/found/</C>.</li>
        <li>A case that still fails stays found, and <C>--keep</C> says why.</li>
      </ul>
      <SrcRef file="tools/cli/tests/keep.rs" label="--keep, tested" />
      <H2>What you write, what gets tested</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Types and invariants: valid values of every type, edge cases first.</li>
        <li><C>require</C> and <C>ensure</C>: many inputs that meet the first, each result checked against the second.</li>
        <li><C>example</C> lines: run as written.</li>
        <li>Capabilities: replaced by fakes, so tests never touch the real world.</li>
      </ul>
      <H2>What nothing checks yet</H2>
      <P>
        Generated tests try every function, but without an <C>ensure</C> or an <C>example</C> they can only check that
        it doesn&apos;t crash, not that its answer is right. So after its results, <C>kavach test</C> says where a
        person should look first:
      </P>
      <Files name="testing/missed" first={["src/orders.kav", "src/billing/invoice.kav"]} terminal={false} />
      <Terminal session={missed.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Weakly tested</strong> lists each function with no <C>ensure</C> or <C>example</C>: <C>label</C> and <C>number</C>. A <C>require</C> limits the inputs but checks no answer, so it doesn&apos;t count.</li>
        <li><strong className="text-ink">Nothing checked by hand</strong> lists each module with no <C>example</C>, <C>ensure</C>, <C>invariant</C> or <C>test</C> at all, in the file or in its <Link href="/docs/errors/testing/test-blocks/">test file beside it</Link>. <C>orders.kav</C> has an <C>example</C>, so only <C>invoice.kav</C> is listed.</li>
        <li>An <C>example</C> line, or an <C>invoice.test.kav</C> beside it, takes a module off the list.</li>
      </ul>
    </>
  ),
  deeper: (
    <>
      <P>The values most likely to break code are tried first, then random ones:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Numbers: <C>0</C>, <C>1</C>, <C>-1</C>, the smallest and largest, and the values next to every number in a <C>require</C>.</li>
        <li>Text: empty, one character, spaces, non-Latin text, very long.</li>
        <li>Lists, maps and sets: empty, one item, many, duplicates.</li>
        <li>Unions: every variant. Records: generated fields that never break an <C>invariant</C>.</li>
      </ul>
      <P>
        A failing input is shrunk to the smallest one that still fails, and saved under <C>tests/found/</C>. A
        function with no contract and no example is still checked for panics, and listed as weakly tested, so a
        missing contract is a visible choice.
      </P>
    </>
  ),
};

export const fromTypesDoc: LessonDoc = {
  lead: "From a function's types and contracts alone, kavach test makes the inputs and checks the results.",
  simple: <>Like an inspector who reads the label on a machine, then feeds it many different inputs to see if it keeps its promises. You don&apos;t have to think up the inputs yourself.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="testing/sub-contracts.test" />
      <P>Two hundred generated calls, each meeting the <C>require</C> and checked against the <C>ensure</C>, plus the example. The <C>seed</C> line lets anyone repeat the run exactly.</P>
    </>
  ),
};

export const ownValuesDoc: LessonDoc = {
  lead: "When the values kavach test makes up don't suit a type, the type makes its own: it implements Sample, and builds a value from a Gen.",
  simple: <>The test tool usually invents values by itself, like a cook trying random ingredients. For a type with a strict shape, such as an email address, you hand it a recipe instead, and it cooks many values from it.</>,
  minutes: 3,
  before: ["errors/testing/tests-from-types"],
  body: (
    <>
      <Ex name="testing/own-sample.test" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Whenever <C>kavach test</C> needs an <C>Email</C>, it calls <C>sample</C> with a <C>Gen</C>, here 200 times for <C>domain</C>.</li>
        <li>A <C>Gen</C>&apos;s choices are seeded like the rest of the run, so the run can be repeated. It is not the <C>Random</C> capability, so <C>sample</C> needs no <C>uses</C>.</li>
        <li>A value from <C>sample</C> that breaks an <C>invariant</C> is not used, and <C>sample</C> is called again.</li>
        <li><C>sample</C> is tested like any function: its <C>ensure</C> is checked on every value it makes.</li>
      </ul>
      <ModuleTable file="07-test.md" number="14.7" first="Method" />
      <P><C>Sample</C> and <C>Gen</C>, like every standard type, need no <C>use</C>.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/11-testing.md" label="spec 11.3 · generated values" />
        <SrcRef file="docs/spec/14-standard-library/07-test.md" label="spec 14.7 · std.test" />
      </div>
    </>
  ),
};

export const testBlocksDoc: LessonDoc = {
  lead: "A test block checks a story of several steps that contracts can't express.",
  simple: <>Some checks are a little story: first do this, then that, then look at the result. A <C>test</C> block writes down such a story, for what contracts alone can&apos;t describe.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="testing/sub-story.test" />
      <P>
        <C>assert</C> checks a condition. The report also flags <C>total_paid</C> as weakly tested: it has a story
        but no contract, so only some of its behaviour is checked.
      </P>
      <Ex name="testing/sub-random.test" />
      <H2>Tests use nothing unless given</H2>
      <P>
        A test runs with no capabilities unless its name is followed by <C>uses</C>. Reaching one without it,
        directly or through a function, is a compile error:
      </P>
      <Ex name="testing/test-uses.fails" />
      <P>Say what the test needs, and it runs with the fake of it, so nothing real is touched:</P>
      <Ex name="testing/test-uses.test" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The console, the clock and random numbers are always there: a test run captures the output, can freeze the clock and seeds the numbers.</li>
        <li>A test can also hand in its own implementation of a capability: <C>{"test \"refund flow\" with Payments = FakePayments() { … }"}</C>.</li>
      </ul>
      <SrcRef file="tests/conformance/11-testing/tests-use-nothing-unless-given.kav" label="tested" />
      <H2>Where test blocks go</H2>
      <P>
        Beside the code they test: the tests of <C>orders.kav</C> go in <C>orders.test.kav</C>, in the same folder.
        A glance at a folder shows which modules have hand-written tests.
      </P>
      <Files name="testing/beside" first={["src/orders.kav", "src/orders.test.kav"]} terminal={false} />
      <Terminal session={beside.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>orders.test.kav</C> is part of the module <C>orders</C>: it uses the private <C>fee</C>, with no <C>use</C>.</li>
        <li><C>kavach run</C> leaves the tests out; <C>kavach test</C> runs them with the examples and the generated tests.</li>
        <li>A story across many modules goes in the project&apos;s <C>tests/</C> folder instead, and a test block may also sit in the file it tests.</li>
        <li>A test file must be named after a module beside it. Otherwise it is an error that says so:</li>
      </ul>
      <Files name="testing/orphan.fails" first={["src/invoice.test.kav"]} outcome="refused" />
    </>
  ),
};

export const fakesDoc: LessonDoc = {
  lead: "In a test, files, the network, the clock and your own capabilities are fakes: nothing real is touched.",
  simple: <>Like a flight simulator, where pilots practise without a real plane. In a test, files, the network and the clock are pretend copies, so nothing real is changed, and a test can even try a full disk.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="testing/fake-fs.test" />
      <P>
        Each of the 200 cases gets an empty file system in memory. Some cases find the disk full, so the error path
        is tested too, without you writing it. The network fake answers with successes, timeouts and refusals; the
        clock starts at a fixed date, and <C>sleep</C>, <C>after</C> and each round of <C>every</C> move it on at
        once, taking no time.
      </P>
      <P>
        <C>Env</C> is a fake too: <C>env(name)</C> reads no real variable. Each name is decided the first time a case
        reads it: not set, so the call fails, or set to generated text (empty, a word, a number, a URL or{" "}
        <C>true</C>). It stays that way for the rest of the case, and is a secret, as a real one is.
      </P>
      <SrcRef file="tests/conformance/11-testing/env-fake-stays-the-same-in-a-case.kav" label="Env fake, tested" />
    </>
  ),
};
