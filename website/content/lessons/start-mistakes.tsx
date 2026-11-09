import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { mistakes, mistakeSteps, type Mistake } from "@/lib/mistakes";
import { H2, P } from "./parts";

// Every row comes from benchmarks/mistakes/README.md (lib/mistakes.ts),
// and the site check runs each case again with `kavach check`.

/** README text with its `code` spans shown as code. */
const code = (text: string) => text.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));

const stepColor: Record<string, string> = {
  "compile error": "bg-bad-bg text-bad",
  "compile error, auto-fix": "bg-bad-bg text-bad",
  warning: "bg-chip text-ink",
  "warning, auto-fix": "bg-ok-bg text-ok",
  "only when it runs": "bg-chip text-muted",
};

const Step = ({ step }: { step: string }) => (
  <span className={`whitespace-nowrap rounded-full px-2.5 py-0.5 text-[12px] font-bold ${stepColor[step] ?? "bg-chip text-ink"}`}>{step}</span>
);

const isError = (m: Mistake) => m.step.startsWith("compile error");
const hasAutoFix = (m: Mistake) => m.step.endsWith("auto-fix");

/** How many mistakes each step caught, worked out from the rows. */
function Tally({ all }: { all: Mistake[] }) {
  const before = all.filter((m) => m.step !== "only when it runs").length;
  const fixed = all.filter(hasAutoFix).length;
  const steps = [...new Set([...all.map((m) => m.step), "only when it runs"])];
  return (
    <div className="flex flex-col gap-3 rounded-xl border border-line bg-panel px-5 py-4">
      <p className="m-0 text-[17px]">
        <strong className="text-ink">{before} of {all.length}</strong> caught before the program runs
        {fixed > 0 && <>, and <strong className="text-ink">{fixed}</strong> of them with an automatic fix</>}.
      </p>
      <div className="flex flex-wrap gap-2">
        {steps.map((step) => (
          <span key={step} className="flex items-center gap-1.5 text-[14px] text-muted">
            <Step step={step} /> {all.filter((m) => m.step === step).length}
          </span>
        ))}
      </div>
    </div>
  );
}

export const mistakesDoc: LessonDoc = {
  lead: "Everyday mistakes, each one a small program, and the step that catches it before the program runs, with the message it gives.",
  simple: <>Everyone makes small slips, like a misspelled name or a missing case. Kavach works like a spell checker for programs: it catches many of these before the program runs, and its message says what went wrong.</>,
  minutes: 6,
  before: ["start/classic-mistakes-fixed"],
  body: (
    <>
      <P>
        A mistake found while you type costs seconds. The same mistake found by a customer costs much more. These are
        the mistakes people make under deadline pressure: a misspelled name, a wrong type, a missing case, an index
        one past the end. Each was checked with <C>kavach check</C> when this site was built.
      </P>
      <Tally all={mistakes()} />
      <H2>The steps</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            {mistakeSteps().map(({ step, means }) => (
              <tr key={step} className="border-b border-line align-top">
                <td className="py-2 pr-4"><Step step={step} /></td>
                <td className="py-2 text-muted">{code(means)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <H2>Every mistake</H2>
      <P>Open one to see the program and what <C>kavach check</C> says about it.</P>
      <div className="flex flex-col gap-2">
        {mistakes().map((m) => (
          <details key={m.file} className="rounded-lg border border-line bg-panel px-4 py-2.5 [&[open]]:pb-4">
            <summary className="flex cursor-pointer flex-wrap items-center justify-between gap-2 text-[15px] font-semibold text-ink">
              {code(m.title)}
              <Step step={m.step} />
            </summary>
            <div className="mt-3 flex flex-col gap-2">
              <CodeBlock
                code={m.code}
                file={m.file}
                run={`kavach check ${m.file}`}
                output={`${isError(m) ? "error" : "warning"}: ${m.message}`}
                outcome={isError(m) ? "refused" : "warned"}
              />
            </div>
          </details>
        ))}
      </div>
      <P>
        Those marked <em>auto-fix</em> have one right fix: the editor offers it as a quick fix, and{" "}
        <Link href="/docs/tools/the-kavach-command/fix/">kavach fix</Link> applies it to whole folders. For the traps
        other languages are known for, such as <C>0.1 + 0.2</C>, see{" "}
        <Link href="/docs/start/classic-mistakes-fixed/">Classic Mistakes, Fixed</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="benchmarks/mistakes/README.md" label="the mistakes benchmark" />
      </div>
    </>
  ),
};
