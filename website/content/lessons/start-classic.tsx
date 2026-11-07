import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// The everyday correctness traps of other languages, and what Kavach does
// instead. The research, with its sources, is docs/vision/classic-mistakes.md;
// every Kavach answer here is a sample run by the real compiler.

const trap = (title: string, elsewhere: React.ReactNode, sample: string, after?: React.ReactNode) => (
  <section key={title} className="flex flex-col gap-3">
    <h3 className="m-0 text-lg font-extrabold">{title}</h3>
    <p className="m-0 text-[15px] text-muted">
      <strong className="text-ink">Elsewhere:</strong> {elsewhere}
    </p>
    <Ex name={sample} />
    {after ? <p className="m-0 text-[15px] text-muted">{after}</p> : null}
  </section>
);

// Habits from other languages and what Kavach says to each; every row's
// sample is refused by the compiler with that message.
const habits: [habit: string, from: string, ours: string, sample: string][] = [
  ["a === 1", "JavaScript", "a == 1", "habits/triple-eq.fails"],
  ["null, nil, None", "most languages", "none, in an optional type", "habits/null.fails"],
  ["type Dog: Animal", "Java, C#, Python", "a field, or an interface", "habits/inherit.fails"],
  ["const match = 3", "any", "match_value", "habits/keyword.fails"],
  ["const 2nd_place", "any", "place_2nd", "variables/digit.fails"],
  ["prices.sum()", "Python, JavaScript", "prices.sum(p => p)", "habits/sum.fails"],
  ["s.slice(-4)", "Python, JavaScript", "s.slice(s.len() - 4, s.len())", "habits/slice.fails"],
  ["(inout n) => …", "C#, Swift", "a named fn with inout", "habits/lambda-inout.fails"],
  ['log.info("…")', "most languages", "a LogEvent record", "habits/log-text.fails"],
];

export const classicDoc: LessonDoc = {
  lead: "0.1 + 0.2 is 0.3, a NaN can't break a sort, \"😀\" is one character long: the everyday traps of other languages, fixed.",
  simple: <>Every language has hidden traps: the program runs and gives an answer, but the answer is quietly wrong. Kavach fixes the best-known ones, or stops with a clear message instead of carrying on with a wrong answer.</>,
  minutes: 8,
  body: (
    <>
      <P>
        Every language has traps where a program runs, gives an answer, and the answer is wrong. Here are the best
        known, what other languages do, and what Kavach does. Every Kavach answer below was printed by the real
        compiler when this site was built.
      </P>

      <H2>Numbers</H2>
      {trap(
        "0.1 + 0.2",
        <>C, Java, JavaScript, Python and Rust all print <C>0.30000000000000004</C>.</>,
        "classic/constants",
        <>
          The compiler works numbers written in the code out exactly, then rounds once. Values computed while the
          program runs still round at each step, as on every computer: for money, use{" "}
          <Link href="/docs/basics/data-types/decimal/">Decimal</Link>, which is always exact and has no size limit.
        </>,
      )}
      {trap(
        "Integers that wrap around",
        <>
          C, C++, Java and Go wrap silently. The JDK&apos;s binary search broke on huge arrays because{" "}
          <C>(low + high) / 2</C> overflowed into a negative index.
        </>,
        "classic/midpoint.panics",
        <>
          Overflow is checked in every build, so the bug stops at its line instead of giving a wrong index. When the
          numbers are known while compiling, as in <C>(1_500_000_000 + 2_000_000_000) / 2</C> on <C>Int32</C>, it
          doesn&apos;t even compile: &quot;this always panics&quot;.
        </>,
      )}
      {trap(
        "A NaN in a list breaks sorting",
        <>C++ has undefined behaviour; Java, JavaScript and Python leave the list half sorted.</>,
        "classic/nan-sort",
        <>A <C>NaN</C> (&quot;not a number&quot;) sorts after every number, in <C>sort</C>, <C>sort_by</C>, <C>min</C> and <C>max</C>.</>,
      )}
      {trap(
        "NaN turned into a whole number",
        <>Undefined behaviour in C; Java and Rust quietly give <C>0</C>.</>,
        "classic/nan-int.panics",
      )}
      {trap(
        "\"1\" + 1",
        <>JavaScript gives <C>&quot;11&quot;</C>, and <C>&quot;10&quot; * &quot;2&quot;</C> gives <C>20</C>.</>,
        "classic/no-coercion.fails",
        <>Types never change behind your back; convert on purpose with <C>String(1)</C> or <C>Int(&quot;1&quot;)</C>.</>,
      )}
      {trap(
        "Text read as a number",
        <>JavaScript&apos;s <C>parseInt(&quot;23sd&quot;)</C> is <C>23</C>.</>,
        "errors/parse",
        <><C>Int(text)</C> reads the whole text or fails, and you say what happens then.</>,
      )}
      <Note>
        Also fixed: <C>x % 2 == 1</C>, which misses negative odd numbers, gets a warning; <C>==</C> between computed
        floats gets a warning pointing to <C>a.near(b)</C>; a constant too big for its type is a compile error that
        names a type that fits. Where C and Python disagree on <C>-7 / 2</C>, Kavach has both: <C>/</C> and <C>%</C>{" "}
        as C, <C>div_floor</C> and <C>mod</C> as Python (see <Link href="/docs/basics/operators/arithmetic/">Arithmetic</Link>).
        See <Link href="/docs/errors/kinds-of-errors/">Kinds of Errors</Link>.
      </Note>

      <H2>Text</H2>
      {trap(
        "Length, capitals and sorting",
        <>
          In JavaScript, Java and C#, <C>&quot;😀&quot;.length</C> is <C>2</C>. JavaScript&apos;s default sort puts{" "}
          <C>[10, 9, 1, 2]</C> in the order <C>[1, 10, 2, 9]</C>.
        </>,
        "classic/text",
        <>Text counts characters; <C>&quot;ß&quot;</C> in capitals is <C>&quot;SS&quot;</C>, as Unicode says; numbers sort as numbers.</>,
      )}

      <H2>Values</H2>
      {trap(
        "A default argument shared between calls",
        <>In Python, <C>def add(x, items=[])</C> keeps one list for every call, so the second call returns <C>[1, 2]</C>.</>,
        "classic/defaults",
      )}
      {trap(
        "A closure that sees the loop's last value",
        <>JavaScript with <C>var</C>, and Go before 1.22, print <C>3 3 3</C>.</>,
        "classic/closures",
        <>A lambda keeps a copy of each value it uses, taken when it is made.</>,
      )}
      {trap(
        "Two names for one list",
        <>In Python, Java and JavaScript, <C>b = a</C> shares the list, so changing <C>b</C> changes <C>a</C>.</>,
        "memory/values",
      )}

      <H2>Habits from other languages</H2>
      <P>
        Code written from habit in another language gets one clear error that says what to write in Kavach, instead
        of a puzzle. Each of these is refused before the program runs:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Habit</th>
              <th className="py-2 pr-4 font-bold">From</th>
              <th className="py-2 font-bold">In Kavach</th>
            </tr>
          </thead>
          <tbody>
            {habits.map(([habit, from, ours]) => (
              <tr key={habit} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap"><C>{habit}</C></td>
                <td className="py-2 pr-4 text-muted">{from}</td>
                <td className="py-2"><C>{ours}</C></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {habits.map(([habit, , , sample]) => (
        <Ex key={habit} name={sample} />
      ))}

      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/vision/classic-mistakes.md" label="the research, with its sources" />
        <SrcRef file="docs/decisions/0043-constants-are-exact.md" label="decision 0043 · exact constants" />
      </div>
    </>
  ),
};
