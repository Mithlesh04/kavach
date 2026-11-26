import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, P } from "./parts";

// ---------- Concepts: the checking choices ----------
//
// Why Kavach checks what it checks: tests from contracts, loops on cores
// that must give the one-core answer, and numbers that never change kind or
// overflow silently. Each page as its decision record makes the choice.

const ul = "m-0 flex list-disc flex-col gap-1.5 pl-5";

export const conceptTestsDoc: LessonDoc = {
  lead: "In Kavach you write the rules a function follows, as contracts, and kavach test writes the tests: it generates inputs from the types, checks every rule, and shrinks a failure to the smallest case.",
  simple: <>Writing a test for every case is slow, so people skip it, and the case nobody thought of is the one that breaks. Kavach asks you for the rules instead, then tries hundreds of cases for you.</>,
  minutes: 6,
  before: ["errors/contracts"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        Testing usually means one hand-written test per case: empty input, a negative number, a service that is
        down. That is slow, so it gets skipped, and the cases nobody thought of are the ones that break in
        production.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Unit tests by hand, in every mainstream language.</li>
        <li>Property-based testing as a library, such as QuickCheck for Haskell and Hypothesis for Python: powerful, and something each team must choose and set up.</li>
        <li>Contracts in a few languages, such as Eiffel and Ada&apos;s SPARK, checked while running or proved.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        Types, contracts and capabilities already say most of what a test needs. So <C>kavach test</C> makes inputs
        from each function&apos;s parameter types, keeps to its <C>require</C>s and <C>invariant</C>s, and checks its{" "}
        <C>ensure</C>s and that it never panics. Here a function that splits a bill passes its example, and the
        generated bills find one that loses a cent:
      </P>
      <Files name="guides/split-first" />
      <ul className={ul}>
        <li>A failing input is shrunk to the smallest one, saved in <C>tests/found/</C>, and replayed on every run.</li>
        <li>An <C>example</C> is a one-line test in the signature, and documentation at once.</li>
        <li>Capabilities are faked, so error paths are tested without extra code.</li>
        <li><C>test</C> blocks stay, for stories of several steps.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>Generated tests are only as strong as the contracts. So the report lists weakly tested functions by name, instead of a coverage percentage that would hide them.</li>
        <li>Some values can&apos;t be generated yet, such as tensors; the report says so, and examples check them.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0010-tests-from-contracts.md" label="decision 0010 · tests from contracts" />
        <SrcRef file="docs/spec/11-testing.md" label="spec 11 · testing" />
      </div>
      <P>
        The whole story, from the bug to the fix to the kept example:{" "}
        <Link href="/docs/guides/test-with-contracts-and-generated-tests/">Test with Contracts and Generated Tests</Link>.
      </P>
    </>
  ),
};

export const conceptOneCoreDoc: LessonDoc = {
  lead: "A loop runs on one core unless it says on cores. Then its rounds run on every core, and the compiler checks that the answer is exactly the one-core answer, so using more cores never changes what a program does.",
  simple: <>Splitting work between helpers is faster, but only if nobody writes on someone else&apos;s page. Kavach lets a loop share its work out between cores, and checks that each round writes only its own part.</>,
  minutes: 5,
  before: ["speed/loops-on-all-cores"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        Work spread over cores is faster, and easy to get wrong: two threads write the same place, or add up floats
        in a different order, and the answer changes from run to run. Such bugs hide in tests and show up under load.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Threads and locks by hand, as in C, Java and Go, where a data race is the programmer&apos;s to avoid.</li>
        <li>A parallel loop from a library, such as OpenMP or Rust&apos;s Rayon. Rust&apos;s types rule out data races, but not a different order of adding floats.</li>
        <li>Some compilers spread a loop on their own, so the code doesn&apos;t say which loops do.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        Every loop runs on one core, in order, as in C, Go, Rust and JavaScript, so code that reads the same costs the
        same. <C>on cores</C> before a loop shares its rounds between the cores, and the compiler allows only what
        keeps the one-core answer: each round writes its own slot, its own row, or adds whole numbers into a total.
      </P>
      <Ex name="speed/cores-rows" />
      <P>Anything else is a compile error that says why, such as a round writing a place another might reach:</P>
      <Ex name="guides/cores-own-slot.fails" />
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>Some loops can&apos;t take <C>on cores</C> as written: a float <C>+=</C>, a <C>print</C> inside. Rewrite them so each round has its own slot.</li>
        <li>You decide which loops use the cores. The compiler never decides that silently.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0126-loops-on-one-core-unless-on-cores.md" label="decision 0126 · on cores" />
        <SrcRef file="docs/spec/04-control-flow.md" label="spec 4.9 · where a loop runs" />
      </div>
      <P>
        A whole program, run on one core, every core and the GPU:{" "}
        <Link href="/docs/guides/use-every-core-and-the-gpu/">Use Every Core and the GPU</Link>.
      </P>
    </>
  ),
};

export const conceptNumbersDoc: LessonDoc = {
  lead: "A number in Kavach never changes kind on its own, and never wraps around silently. Every conversion is written, and a result that doesn't fit is a panic at the line that made it.",
  simple: <>In some languages, 2 billion plus 2 billion can quietly become a negative number, and &quot;1&quot; + 1 can quietly become &quot;11&quot;. Kavach refuses both: it either does exactly what you wrote or stops and says why.</>,
  minutes: 5,
  before: ["basics/data-types"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        Silent conversions and silent overflow give wrong answers that look right. A sum of money wraps to a negative
        number; a text and a number join into nonsense; a float rounds a price. Nothing stops the program, so the
        wrong number travels on.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>JavaScript turns <C>&quot;1&quot; + 1</C> into <C>&quot;11&quot;</C>, and every number is a float.</li>
        <li>C wraps unsigned numbers around, and leaves signed overflow undefined.</li>
        <li>Java and Go wrap around silently. Rust checks in debug builds and wraps in release builds.</li>
        <li>Python&apos;s whole numbers never overflow, and <C>1 / 2</C> quietly gives a float.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>There are no hidden conversions. Text and a number don&apos;t mix until you say how:</P>
      <Ex name="classic/no-coercion.fails" />
      <P>A whole-number result that doesn&apos;t fit its type is a panic, at the line that made it:</P>
      <Ex name="classic/midpoint.panics" />
      <P>Constants are worked out exactly while compiling, so the old float surprise doesn&apos;t happen there:</P>
      <Ex name="classic/constants" />
      <ul className={ul}>
        <li>Every conversion has one form, the type&apos;s name: <C>Float(n)</C>, <C>Int32(x)</C>.</li>
        <li>When wrapping is meant, as in a hash, say so: <C>wrapping_add</C>. When a missing answer is better than a panic: <C>checked_add</C>.</li>
        <li>Money is a <C>Decimal</C>, exact and without a size limit.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>A conversion is a few more characters to write.</li>
        <li>A check on each whole-number operation, which takes a little time. What it costs on real programs is in the <Link href="/docs/benchmarks/">Benchmarks</Link>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0013-core-language-decisions.md" label="decision 0013 · no hidden conversions" />
        <SrcRef file="docs/spec/02-types/01-built-in-types.md" label="spec 2.1 · numbers" />
        <SrcRef file="docs/spec/05-errors.md" label="spec 5.6 · panics" />
      </div>
      <P>
        More traps other languages fall into, each shown fixed:{" "}
        <Link href="/docs/start/classic-mistakes-fixed/">Classic Mistakes, Fixed</Link>.
      </P>
    </>
  ),
};
