import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { conformanceTests, sectionTitles, type ConformanceTest } from "@/lib/conformance";
import { course } from "@/lib/course";
import { findNode } from "@/lib/nav";
import { H2, P } from "./parts";
import { stdModules } from "./reference-std";
import { countsOf, TestRows } from "./tested-table";

// ---------- Reference › What Is Tested ----------
//
// Every conformance test, by spec chapter: the index counts them, and each
// chapter's page lists its tests by section. A section with a reference
// page of its own (a std module, Built-in Methods, Built-in Types) lists
// its tests there, and the chapter's page links to it.

const KEY = "reference/what-is-tested";

/** Sections whose tests are listed on their own reference page, and that page's key. */
const HOSTS = new Map<string, string>([
  ["2.1", "reference/built-in-types"],
  ["14.2", "reference/built-in-methods"],
  ...stdModules.map((m): [string, string] => [m.number, m.key]),
]);

const chapterOf = (t: ConformanceTest) => t.sections[0].split(".")[0];

/** Section numbers in reading order: 1.4 before 1.11, 14.2 before 14.10. */
const byNumber = (a: string, b: string) => {
  const [x, y] = [a.split(".").map(Number), b.split(".").map(Number)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? -1) !== (y[i] ?? -1)) return (x[i] ?? -1) - (y[i] ?? -1);
  return 0;
};

/** The section a test is listed under: its first, or the hosted section that holds it (`14.11` for `14.11.2`). */
const listedUnder = (t: ConformanceTest) =>
  [...HOSTS.keys()].find((n) => t.sections[0] === n || t.sections[0].startsWith(`${n}.`)) ?? t.sections[0];

/** The chapters with tests, each with its subpage of this page (lib/course.ts lists them by the spec's titles). */
const subs = course.find((p) => p.slug === "reference")!.pages.find((p) => p.slug === "what-is-tested")!.subs;
const chapters = [...new Set(conformanceTests.map(chapterOf))].sort(byNumber).map((n) => {
  const title = sectionTitles.get(n);
  const sub = subs.find((s) => s.title === title);
  if (!sub) throw new Error(`What Is Tested: spec chapter ${n} (${title}) has tests but no subpage in lib/course.ts`);
  return { n, title: title!, key: `${KEY}/${sub.slug}`, tests: conformanceTests.filter((t) => chapterOf(t) === n) };
});

function Chapter({ n }: { n: string }) {
  const tests = chapters.find((c) => c.n === n)!.tests;
  const sections = [...new Set(tests.map(listedUnder))].sort(byNumber);
  return (
    <>
      <P>{countsOf(tests)}</P>
      {sections.map((s) => {
        const here = tests.filter((t) => listedUnder(t) === s);
        const host = HOSTS.get(s);
        return (
          <div key={s} className="flex flex-col gap-3">
            <H2>{`${s} ${sectionTitles.get(s) ?? ""}`}</H2>
            {host ? (
              <P>
                {countsOf(here)} {here.length === 1 ? "It is" : "They are"} listed on <Link href={findNode(host)!.href}>{findNode(host)!.title}</Link>.
              </P>
            ) : (
              <TestRows tests={here} />
            )}
          </div>
        );
      })}
    </>
  );
}

function Index() {
  return (
    <>
      <P>
        A conformance test is a small program that checks one rule of the spec. Its first lines say which rule, which
        section, and what must happen: an output, a compile error, a panic. The test runner checks every one, so each
        rule listed here holds for the compiler this site was built with.
      </P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong>Works</strong>: the program runs and prints what the test expects.</li>
        <li>
          <strong>Refused</strong>: wrong use is stopped, by a compile error, a warning, a panic, an exit code other
          than 0, a failing <C>test</C>, or an error value the program prints (<C>{"// Checks: misuse"}</C>).
        </li>
      </ul>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Chapter</th>
              <th className="py-2 pr-4 font-bold">Tests</th>
              <th className="py-2 pr-4 font-bold">Works</th>
              <th className="py-2 font-bold">Refused</th>
            </tr>
          </thead>
          <tbody>
            {chapters.map((c) => {
              const refused = c.tests.filter((t) => t.refused).length;
              return (
                <tr key={c.n} className="border-b border-line align-top">
                  <td className="py-2 pr-4"><Link href={findNode(c.key)!.href}>{`${c.n}. ${c.title}`}</Link></td>
                  <td className="py-2 pr-4">{c.tests.length}</td>
                  <td className="py-2 pr-4 text-muted">{c.tests.length - refused}</td>
                  <td className="py-2 text-muted">{refused}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <P>{countsOf(conformanceTests)}</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/README.md" label="the test format" />
        <SrcRef file="tools/cli/tests/conformance.rs" label="the test runner" />
      </div>
    </>
  );
}

/** This page and its chapters' pages, keyed by their course keys. */
export const testedDocs: Record<string, LessonDoc> = {
  [KEY]: {
    lead: "Every rule the conformance tests check, by spec chapter: what works, and what wrong use is refused.",
    minutes: 3,
    body: <Index />,
  },
  ...Object.fromEntries(
    chapters.map((c) => [
      c.key,
      {
        lead: `The conformance tests of spec chapter ${c.n}, ${c.title}, by section.`,
        minutes: 5,
        body: <Chapter n={c.n} />,
      } satisfies LessonDoc,
    ]),
  ),
};
