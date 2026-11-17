import { C } from "@/components/inline-code";
import versions from "@/data/versions.json";
import { type ConformanceTest, testsOf } from "@/lib/conformance";
import { H2, P } from "./parts";
import { Cell } from "./spec-table";

// ---------- What is tested ----------
//
// The conformance tests of one spec section, as a table: each test's rule,
// whether it checks that something works or that wrong use is refused, and
// a link to the test. Read from the tests' headers (lib/conformance.ts).

const current = versions.versions.find((v) => v.id === versions.latest) ?? versions.versions[0];

/** A test's file on GitHub, at this docs version's tag, as SrcRef links it. */
export const testHref = (t: ConformanceTest) => `${versions.repo}/blob/${current.tag}/${t.file}`;

/** "12 tests: 7 check that it works, 5 that wrong use is refused." */
export function countsOf(tests: ConformanceTest[]): string {
  const refused = tests.filter((t) => t.refused).length;
  const one = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  return `${one(tests.length, "test")}: ${tests.length - refused} check that it works, ${refused} that wrong use is refused.`;
}

/** The rows of a list of tests. */
export function TestRows({ tests }: { tests: ConformanceTest[] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Rule</th>
            <th className="py-2 pr-4 font-bold">Spec</th>
            <th className="py-2 pr-4 font-bold">Checks</th>
            <th className="py-2 font-bold">Test</th>
          </tr>
        </thead>
        <tbody>
          {tests.map((t) => (
            <tr key={t.file} className="border-b border-line align-top">
              <td className="py-2 pr-4"><Cell text={t.rule} /></td>
              <td className="py-2 pr-4 whitespace-nowrap text-muted">{t.sections.join(", ")}</td>
              <td className="py-2 pr-4 whitespace-nowrap">
                {t.refused ? <span className="text-muted">refused: {t.outcome}</span> : "works"}
              </td>
              <td className="py-2 font-mono text-xs [overflow-wrap:anywhere]">
                <a href={testHref(t)}>{t.name}</a>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** A section's "What is tested": nothing when no test names it first. */
export function WhatIsTested({ number }: { number: string }) {
  const tests = testsOf(number);
  if (!tests.length) return null;
  return (
    <>
      <H2>What is tested</H2>
      <P>
        Each conformance test in <C>tests/conformance/</C> checks one rule of spec {number}. {countsOf(tests)}
      </P>
      <TestRows tests={tests} />
    </>
  );
}
