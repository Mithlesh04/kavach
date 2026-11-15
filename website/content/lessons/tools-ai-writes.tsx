import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { fileFigure, fileTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { Cell, Table } from "./spec-table";

// ---------- How Well AI Writes Kavach ----------
//
// The AI-writability benchmark (benchmarks/ai-writes/), read at build time.
// Every number, table and finding comes from its README, never typed here,
// so the page shows what was measured, where Kavach does badly too.

const README = "benchmarks/ai-writes/README.md";

/** The text of a `## name` section of the README, up to the next one. */
const section = (name: string) =>
  fileFigure(README, new RegExp(`\\n## ${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\n\\n([\\s\\S]*?)(?=\\n## |$)`));

/** The paragraphs of a section: text that is not a table, a list or code. */
const paragraphs = (text: string) =>
  text
    .replace(/```[\s\S]*?```/g, "")
    .split("\n\n")
    .filter((p) => p.trim() && !/^(\||- |```)/.test(p.trim()))
    .map((p) => p.replace(/\n/g, " "));

const bullets = (text: string) => text.split("\n").filter((l) => l.startsWith("- ")).map((l) => l.slice(2));

/** The commands of each code block, with its blank lines left out. */
const codeBlocks = (text: string) =>
  [...text.matchAll(/```\n([\s\S]*?)\n```/g)].map((m) => m[1].split("\n").filter((l) => l.trim()).join("\n"));

/** A section's table, found by its heading, with the same columns as the others. */
function result(heading: string) {
  const t = fileTables(README).find((x) => x.heading === heading);
  if (!t) throw new Error(`${README}: no table under ## ${heading}`);
  if (t.header[0] !== "Model") throw new Error(`${README}: the ${heading} table changed shape`);
  return t;
}

/** A finding: its bold opening, then the rest, with code and links as the README writes them. */
function Finding({ text }: { text: string }) {
  const m = text.match(/^\*\*(.+?)\*\*\s*(.*)$/);
  if (!m) return <Cell text={text} />;
  return (
    <>
      <strong className="text-ink">{m[1]}</strong> <Cell text={m[2]} />
    </>
  );
}

/** Pass@1 as bars: one row per model, one bar per condition, its width the share that passed. */
function PassBars() {
  const { header, rows } = result("Pass@1");
  const conditions = header.slice(1);
  return (
    <div className="flex flex-col gap-4">
      {rows.map(([model, ...cells]) => (
        <div key={model} className="flex flex-col gap-1">
          <div className="text-[15px] font-bold text-ink">{model}</div>
          {cells.map((cell, i) => {
            const share = Number(cell.match(/^(\d+)%/)?.[1]);
            if (Number.isNaN(share)) throw new Error(`${README}: Pass@1 cell "${cell}" has no share`);
            return (
              <div key={conditions[i]} className="grid grid-cols-[4.5rem_1fr_6.5rem] items-center gap-2 text-[14px]">
                <C>{conditions[i]}</C>
                <div className="h-3 rounded bg-track" aria-hidden>
                  <div className="h-3 rounded bg-accent" style={{ width: `${share}%` }} />
                </div>
                <span className="text-muted">{cell}</span>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

/** Every table under a heading, in order, for a section with more than one. */
function tablesUnder(heading: string) {
  const ts = fileTables(README).filter((x) => x.heading === heading);
  if (!ts.length || ts.some((t) => t.header[0] !== "Model")) throw new Error(`${README}: the ${heading} tables changed shape`);
  return ts;
}

function ResultTable({ heading }: { heading: string }) {
  const { header, rows } = result(heading);
  return <Table header={header} rows={rows} />;
}

const conditions = () => {
  const t = fileTables(README).find((x) => x.header.join("|") === "Condition|What the model gets");
  if (!t) throw new Error(`${README}: no Condition|What the model gets table`);
  return t;
};

const intro = () => fileFigure(README, /\n\n(\d+ small programming tasks[^\n]+)\n/);

export const aiWritesDoc: LessonDoc = {
  lead: "How often Claude models write Kavach that runs right: alone, with the guide for AI tools, with the compiler's errors sent back, and with Kavach's own tools, beside the same tasks in Python. Measured, failures included.",
  simple: <>A new language is only useful if AI assistants can write it. So each model was given the same small tasks, and each answer was run and checked. On their own, the models mostly wrote another language by habit; with a short guide and the compiler&apos;s messages, nearly every answer worked.</>,
  minutes: 6,
  before: ["tools/tools-for-ai"],
  body: (
    <>
      <P>
        <Cell text={intro()} />
      </P>
      <Table header={conditions().header} rows={conditions().rows} />
      <H2>How many passed</H2>
      <P>
        <Cell text={paragraphs(section("Pass@1"))[0]} />
      </P>
      <PassBars />
      <ResultTable heading="Pass@1" />
      <H2>What the run shows</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {bullets(section("What the run shows")).map((b) => (
          <li key={b}><Finding text={b} /></li>
        ))}
      </ul>
      {paragraphs(section("What the run shows")).map((p) => (
        <P key={p}><Cell text={p} /></P>
      ))}
      <P>
        This is the message <C>let mut</C>, where most zero-shot answers stopped, now gets:
      </P>
      <Ex name="habits/let-mut.fails" />
      <H2>Between the two runs</H2>
      <P><Cell text={paragraphs(section("Between the two runs"))[0]} /></P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {bullets(section("Between the two runs")).map((b) => (
          <li key={b}><Cell text={b} /></li>
        ))}
      </ul>
      <ResultTable heading="Between the two runs" />
      {paragraphs(section("Between the two runs")).slice(1).map((p) => (
        <P key={p}><Cell text={p} /></P>
      ))}
      <H2>Against Python</H2>
      <P><Cell text={paragraphs(section("Against Python"))[0]} /></P>
      {tablesUnder("Against Python").map((t, i) => (
        <div key={t.header.join()} className="flex flex-col gap-3">
          {i > 0 ? <P><Cell text={paragraphs(section("Against Python"))[1]} /></P> : null}
          <Table header={t.header} rows={t.rows} />
        </div>
      ))}
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {bullets(section("Against Python")).map((b) => (
          <li key={b}><Cell text={b} /></li>
        ))}
      </ul>
      <H2>Tries, cost and time</H2>
      <P><Cell text={paragraphs(section("Repair rounds"))[0]} /></P>
      <ResultTable heading="Repair rounds" />
      <P><Cell text={paragraphs(section("Cost and time"))[0]} /></P>
      <ResultTable heading="Cost and time" />
      <H2>What went wrong</H2>
      <P><Cell text={paragraphs(section("Failure kinds"))[0]} /></P>
      <ResultTable heading="Failure kinds" />
      <H2>Found while writing the tasks</H2>
      <P><Cell text={paragraphs(section("Found while writing the tasks"))[0]} /></P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {bullets(section("Found while writing the tasks")).map((b) => (
          <li key={b}><Cell text={b} /></li>
        ))}
      </ul>
      {paragraphs(section("Found while writing the tasks")).slice(1).map((p) => (
        <P key={p}><Cell text={p} /></P>
      ))}
      <Note>
        Each cell was run once, so a small difference between two models may not hold in another run. The tasks,
        prompts and grading are in <C>method.md</C>, linked below.
      </Note>
      <H2>Run it yourself</H2>
      <P><Cell text={paragraphs(section("Run it yourself"))[0]} /></P>
      <Terminal commands session={codeBlocks(section("Run it yourself"))[0]} />
      {paragraphs(section("Run it yourself")).slice(1, 3).map((p) => (
        <P key={p}><Cell text={p} /></P>
      ))}
      <Terminal commands session={codeBlocks(section("Run it yourself"))[1]} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {bullets(section("Run it yourself")).map((b) => (
          <li key={b}><Cell text={b} /></li>
        ))}
      </ul>
      <P><Cell text={paragraphs(section("Run it yourself"))[3]} /></P>
      <P>
        The guide the <C>docs</C> condition gives the model is the site&apos;s own{" "}
        <Link href="/llms.txt">llms.txt</Link>, and the tools are <Link href="/docs/tools/tools-for-ai/mcp/">kavach mcp</Link>&apos;s.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={README} label="the benchmark" />
        <SrcRef file="benchmarks/ai-writes/method.md" label="how it works" />
        <SrcRef file="benchmarks/ai-writes/results/2026-11-15-kavach.json" label="every answer" />
        <SrcRef file="benchmarks/ai-writes/results/2026-11-14-kavach.json" label="the first run" />
        <SrcRef file="benchmarks/ai-writes/results/2026-11-15-python.json" label="Python's answers" />
        <SrcRef file="benchmarks/ai-writes/languages/python/README.md" label="the Python tasks" />
        <SrcRef file="benchmarks/ai-writes/summarize.py" label="summarize.py" />
        <SrcRef file="benchmarks/ai-writes/tasks/tasks.json" label="the tasks" />
      </div>
    </>
  ),
};
