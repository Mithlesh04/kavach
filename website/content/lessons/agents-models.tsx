import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { agentAttacks, agentFaults, agentModels, type ModelBlock } from "@/lib/agent-bench";
import { H2, P } from "./parts";

// The agent cases on several models (benchmarks/agents/README.md, "Several
// models"): every table, sentence and number is the README's, read at build
// time by lib/agent-bench.ts. The chart is drawn from the agentic loop's table.

/** README text with its `code` spans shown as code and its **bold** start shown bold. */
function code(text: string) {
  const bold = text.match(/^\*\*(.+?)\*\*(.*)$/);
  const spans = (t: string) => t.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));
  return bold ? <><strong className="text-ink">{spans(bold[1])}</strong>{spans(bold[2])}</> : spans(text);
}

const num = (s: string) => Number(s.replace(/[,$\s]|s$/g, ""));
const isKavach = (row: string[]) => row.some((c) => c.startsWith("Kavach"));

function Table({ b }: { b: ModelBlock }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {b.header.map((h) => <th key={h} className="py-2 pr-4 font-bold">{h}</th>)}
          </tr>
        </thead>
        <tbody>
          {b.rows.map((r) => (
            <tr key={r.join("|")} className={`border-b border-line ${isKavach(r) ? "text-ink" : "text-muted"}`}>
              {r.map((cell, i) => <td key={i} className={`py-2 pr-4 ${i ? "whitespace-nowrap" : ""}`}>{code(cell)}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Block({ b, chart }: { b: ModelBlock; chart?: React.ReactNode }) {
  return (
    <>
      <h3 className="m-0 mt-2 text-[17px] font-bold text-ink">{code(b.title)}</h3>
      {b.paragraphs.map((p) => <P key={p}>{code(p)}</P>)}
      {chart}
      <Table b={b} />
      {b.bullets.length > 0 && (
        <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
          {b.bullets.map((l) => <li key={l}>{code(l)}</li>)}
        </ul>
      )}
    </>
  );
}

/** A table's tokens on each model, Kavach's against the other ways', as bars, with the cost at the end. */
function TokenBars({ b, what }: { b: ModelBlock; what: string }) {
  const col = (h: string) => b.header.indexOf(h);
  const rows = b.rows.map((r) => ({ model: r[col("Model")].replace(/`/g, ""), way: r[col("Way")], tokens: num(r[col("Tokens")]), cost: r[col("Cost")], kavach: isKavach(r) }));
  const max = Math.max(...rows.map((r) => r.tokens));
  const models = [...new Set(rows.map((r) => r.model))];
  return (
    <div className="flex flex-col gap-3" role="img" aria-label={`Tokens ${what}: ${rows.map((r) => `${r.model}, ${r.way}: ${r.tokens.toLocaleString("en-US")}`).join("; ")}`}>
      <span className="text-[13px] text-soft">Tokens, input and output, then what they cost:</span>
      {models.map((m) => (
        <div key={m} className="flex flex-col gap-1">
          <span className="font-mono text-[12.5px] text-soft">{m}</span>
          {rows.filter((r) => r.model === m).map((r) => (
            <div key={r.way} className="grid grid-cols-[minmax(8rem,13rem)_1fr] items-center gap-3 text-[13px]">
              <span className={r.kavach ? "font-semibold text-ink" : "text-muted"}>{r.way}</span>
              <span className="flex items-center gap-2">
                <span className={`h-3 rounded ${r.kavach ? "bg-accent" : "bg-line"}`} style={{ width: `${Math.max(2, (r.tokens / max) * 100)}%` }} />
                <span className="font-mono text-[12px] whitespace-nowrap text-soft">{r.tokens.toLocaleString("en-US")} · {r.cost}</span>
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

/** Every case with a provider that fails on purpose: the README's words and table, and the items each way lost, as bars. */
export function ProviderFails() {
  const b = agentFaults();
  const col = (h: string) => b.header.indexOf(h);
  const lost = b.rows.map((r) => ({ way: r[col("Way")], lost: num(r[col("Lost")]), kavach: isKavach(r) }));
  const max = Math.max(1, ...lost.map((l) => l.lost));
  return (
    <section aria-label={b.title} className="flex flex-col gap-3">
      <H2>{b.title}</H2>
      {b.paragraphs.map((p) => <P key={p}>{code(p)}</P>)}
      <div className="flex flex-col gap-1.5" role="img" aria-label={`Items lost: ${lost.map((l) => `${l.way} ${l.lost}`).join(", ")}`}>
        <span className="text-[13px] text-soft">Items that got no answer at all:</span>
        {lost.map((l) => (
          <div key={l.way} className="grid grid-cols-[minmax(9rem,17rem)_1fr] items-center gap-3 text-[13px]">
            <span className={l.kavach ? "font-semibold text-ink" : "text-muted"}>{l.way}</span>
            <span className="flex items-center gap-2">
              <span className={`h-3 rounded ${l.kavach ? "bg-accent" : "bg-line"}`} style={{ width: `${Math.max(1, (l.lost / max) * 100)}%` }} />
              <span className="font-mono text-[12px] text-soft">{l.lost}</span>
            </span>
          </div>
        ))}
      </div>
      <Table b={b} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {b.bullets.map((l) => <li key={l}>{code(l)}</li>)}
      </ul>
      <P>
        What Kavach does with a busy or dropped call: <Link href="/docs/agents/ask/when-the-ai-fails/">When the AI fails</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="benchmarks/agents/run.py" label="run.py --flaky" />
        <SrcRef file="benchmarks/agents/method.md" label="how the faults are made" />
      </div>
    </section>
  );
}

/** Refund notes that try to steer the money: the README's words and table, and each way's tokens, as bars. */
export function UnderAttack() {
  const b = agentAttacks();
  return (
    <section aria-label={b.title} className="flex flex-col gap-3">
      <H2>{b.title}</H2>
      {b.paragraphs.map((p) => <P key={p}>{code(p)}</P>)}
      <TokenBars b={b} what="under attack, first run" />
      <Table b={b} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        {b.bullets.map((l) => <li key={l}>{code(l)}</li>)}
      </ul>
      <P>
        How a type keeps the model&apos;s answer to one of a few actions: <Link href="/docs/agents/ask/typed-answers/">Typed answers</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="benchmarks/agents/data/attacks.json" label="the twenty notes" />
        <SrcRef file="benchmarks/agents/run.py" label="run.py" />
        <SrcRef file="benchmarks/agents/method.md" label="how Paid wrongly is counted" />
        <SrcRef file="benchmarks/agents/results/2026-11-09-claude-haiku-5-5.json" label="every answer, on Haiku" />
        <SrcRef file="benchmarks/agents/results/2026-11-09-claude-opus-5-5.json" label="every answer, on Opus" />
      </div>
    </section>
  );
}

/** The "Several models" section of Agents in Practice. */
export function SeveralModels() {
  const { intro, models, loop, crash, runs } = agentModels();
  return (
    <section aria-label="Several models" className="flex flex-col gap-3">
      <H2>Several models</H2>
      {intro.map((p) => <P key={p}>{code(p)}</P>)}
      {models.map((b) => <Block key={b.title} b={b} />)}
      <Block b={loop} chart={<TokenBars b={loop} what="on the agentic loop's cases" />} />
      <Block b={crash} />
      <Block b={runs} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="benchmarks/agents/run.py" label="run.py --model" />
        <SrcRef file="benchmarks/agents/method.md" label="how several models are run" />
        <SrcRef file="benchmarks/agents/results/2026-11-08-claude-haiku-5-5.json" label="every answer, on Haiku" />
        <SrcRef file="benchmarks/agents/results/2026-11-08-claude-fable-5-1.json" label="every answer, on Fable" />
      </div>
    </section>
  );
}
