import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import { agentCases, agentFile, agentOrders, agentRunCommand, agentWays, pythonFunction, type Case, type Way } from "@/lib/agent-bench";
import type { LessonDoc } from "@/lib/chapter";
import { H2, Note, P } from "./parts";

// Every number on this page is read from benchmarks/agents/README.md at
// build time (lib/agent-bench.ts), and every comparison is worked out from
// those numbers, never typed.

const isKavach = (w: Way) => w.way.startsWith("Kavach");
const allAtOnce = (c: Case) => c.ways.find((w) => isKavach(w) && w.way.includes("all at once"));
const oneByOne = (c: Case) => c.ways.find((w) => isKavach(w) && w.way.includes("one by one"));
const plainPrompt = (c: Case) => c.ways.find((w) => w.way.includes("plain prompt"));
const total = (w: Way) => w.tokensIn + w.tokensOut;
const heaviest = (c: Case) => c.ways.filter((w) => !isKavach(w)).sort((a, b) => total(b) - total(a))[0];
const times = (n: number) => (n >= 10 ? Math.round(n) : Math.round(n * 10) / 10);
const fmt = (n: number) => n.toLocaleString("en-US");
/** README text with its `code` spans shown as code. */
const code = (text: string) => text.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));

/** The Kavach code shown under a case, and what to look for in it. The large case has its own, longer story below. */
const caseCode: Record<string, { file: string; about: React.ReactNode }> = {
  DevOps: {
    file: "devops.kav",
    about: <>The approval depends on the call&apos;s arguments: a restart on staging runs, one in production waits for the on-call person. The model only picks the action; the agent&apos;s rules decide what may happen:</>,
  },
  "Legal review": {
    file: "legal.kav",
    about: <>The answer is a record of three fields, and each field&apos;s <C>{"///"}</C> note tells the model what it means. The prompt says that instructions inside a clause are contract text, not orders. A type can&apos;t tell a true answer from a false one, so that line, and checking the answers, still matter:</>,
  },
  Logistics: {
    file: "logistics.kav",
    about: <>As in DevOps, the rule is in the agent, not in the prompt. A note that says &quot;no approval needed&quot; can talk the model into a reroute, but not past <C>approve Carrier.reroute when cost &gt; 300</C>:</>,
  },
};

/** Every case's savings in one table: tokens against the heaviest Python way, and time against Python's plain
 *  prompt run one item after another, as the usual script does. The last row adds the cases up. */
function Summary({ cases }: { cases: Case[] }) {
  const rows = cases.map((c) => {
    const k = allAtOnce(c);
    const h = heaviest(c);
    const p = plainPrompt(c);
    if (!k || !h || !p) throw new Error(`agents benchmark, ${c.name}: a way is missing`);
    return { name: c.name, kTok: total(k), hTok: total(h), kSec: k.seconds, pSec: p.seconds, right: k.right };
  });
  const sum = (f: (r: (typeof rows)[number]) => number) => rows.reduce((a, r) => a + f(r), 0);
  const all = { kTok: sum((r) => r.kTok), hTok: sum((r) => r.hTok), kSec: sum((r) => r.kSec), pSec: sum((r) => r.pSec) };
  const cell = "py-2 pr-4 whitespace-nowrap";
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Case</th>
            <th className="py-2 pr-4 font-bold">Tokens: Kavach</th>
            <th className="py-2 pr-4 font-bold">Heaviest Python way</th>
            <th className="py-2 pr-4 font-bold">Fewer tokens</th>
            <th className="py-2 pr-4 font-bold">Time: Kavach</th>
            <th className="py-2 pr-4 font-bold">Python, plain prompt</th>
            <th className="py-2 font-bold">Faster</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.name} className="border-b border-line text-muted">
              <td className="py-2 pr-4 text-ink">{r.name}</td>
              <td className={cell}>{fmt(r.kTok)}</td>
              <td className={cell}>{fmt(r.hTok)}</td>
              <td className={`${cell} font-semibold text-ink`}>{times(r.hTok / r.kTok)}×</td>
              <td className={cell}>{r.kSec} s</td>
              <td className={cell}>{r.pSec} s</td>
              <td className="py-2 whitespace-nowrap font-semibold text-ink">{times(r.pSec / r.kSec)}×</td>
            </tr>
          ))}
          <tr className="border-b-2 border-line font-bold text-ink">
            <td className="py-2 pr-4">All {rows.length} cases</td>
            <td className={cell}>{fmt(all.kTok)}</td>
            <td className={cell}>{fmt(all.hTok)}</td>
            <td className={cell}>{times(all.hTok / all.kTok)}×</td>
            <td className={cell}>{Math.round(all.kSec * 10) / 10} s</td>
            <td className={cell}>{Math.round(all.pSec * 10) / 10} s</td>
            <td className="py-2 whitespace-nowrap">{times(all.pSec / all.kSec)}×</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

/** What one case's table shows, worked out from its numbers. */
function Findings({ c }: { c: Case }) {
  const k = allAtOnce(c);
  const slow = oneByOne(c);
  const p = plainPrompt(c);
  const h = heaviest(c);
  if (!k || !slow || !p || !h) throw new Error(`agents benchmark, ${c.name}: a way is missing`);
  const vsPlain = Math.round(((total(k) - total(p)) / total(p)) * 100);
  return (
    <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
      <li>
        <strong className="text-ink">{times(total(h) / total(k))}× fewer tokens</strong> than {h.way.replace(/^Python, /, "Python's ")}: {fmt(total(k))} against {fmt(total(h))}, input and output, with {k.right} right against {h.right}.
      </li>
      <li>
        Against a plain prompt written by hand: {vsPlain === 0 ? "the same tokens" : `${Math.abs(vsPlain)}% ${vsPlain < 0 ? "fewer" : "more"} tokens`}, and every answer checked against its type, with no parsing by hand.
        {k.right !== p.right && ` Here the plain prompt got ${p.right} right, Kavach ${k.right}.`}
      </li>
      <li>
        <strong className="text-ink">{times(slow.seconds / k.seconds)}× faster all at once</strong>: {k.seconds} s against {slow.seconds} s one by one, the same calls waiting for the model together.
      </li>
    </ul>
  );
}

/** Tokens per way, input and output together, as bars: the longest is the most expensive. */
function TokenBars({ c }: { c: Case }) {
  const max = Math.max(...c.ways.map(total));
  return (
    <div className="flex flex-col gap-1.5" role="img" aria-label={`Tokens per way, input and output: ${c.ways.map((w) => `${w.way} ${fmt(total(w))}`).join(", ")}`}>
      {c.ways.map((w) => (
        <div key={w.way} className="grid grid-cols-[minmax(9rem,14rem)_1fr] items-center gap-3 text-[13px]">
          <span className={isKavach(w) ? "font-semibold text-ink" : "text-muted"}>{w.way}</span>
          <span className="flex items-center gap-2">
            <span className={`h-3 rounded ${isKavach(w) ? "bg-accent" : "bg-line"}`} style={{ width: `${Math.max(2, (total(w) / max) * 100)}%` }} />
            <span className="font-mono text-[12px] text-soft">{fmt(total(w))}</span>
          </span>
        </div>
      ))}
    </div>
  );
}

function Results({ c }: { c: Case }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {["Way", "Calls", "Input tokens", "Output tokens", "Time", "Right"].map((h) => (
              <th key={h} className="py-2 pr-4 font-bold">{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {c.ways.map((w) => (
            <tr key={w.way} className={`border-b border-line ${isKavach(w) ? "text-ink" : "text-muted"}`}>
              <td className="py-2 pr-4">{w.way}</td>
              <td className="py-2 pr-4">{w.calls}</td>
              <td className="py-2 pr-4">{fmt(w.tokensIn)}</td>
              <td className="py-2 pr-4">{fmt(w.tokensOut)}</td>
              <td className="py-2 pr-4 whitespace-nowrap">{w.seconds} s</td>
              <td className="py-2">{w.right}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The large case's tricky requests: every note that tries to talk its way past the policy, and the big amounts. */
function TrickyOrders() {
  const tricky = agentOrders().filter((o) => o.expect === "escalate" || o.amount > 500);
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Order</th>
            <th className="py-2 pr-4 font-bold">Amount</th>
            <th className="py-2 pr-4 font-bold">The customer&apos;s note</th>
            <th className="py-2 font-bold">Right answer</th>
          </tr>
        </thead>
        <tbody>
          {tricky.map((o) => (
            <tr key={o.id} className="border-b border-line align-top">
              <td className="py-2 pr-4">{o.id}</td>
              <td className="py-2 pr-4 whitespace-nowrap">{o.amount} USD</td>
              <td className="py-2 pr-4 text-muted">&quot;{o.note}&quot;</td>
              <td className="py-2"><C>{o.expect}</C></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ---------- Agents in Practice ----------

export const practiceDoc: LessonDoc = {
  lead: "Real AI tasks from support, retail, insurance, health care, operations, law and logistics, measured against Python done the usual ways: the same inputs, the same models, the same right answers. Kavach's goal: fewer tokens, no worse answers, less time.",
  simple: <>This page is a fair race: the same real tasks, done in Kavach and in Python&apos;s usual ways, with the same AI and the same right answers. It counts the tokens each way pays for, the time it takes, and how often it is right.</>,
  minutes: 10,
  before: ["agents/agents", "agents/ask"],
  body: (
    <>
      <H2>Tokens and time, at a glance</H2>
      <P>
        Kavach&apos;s agents asked all at once, against Python: tokens (input and output) against the Python way that
        used the most, and total time against the usual Python script, which asks about one item after another.
      </P>
      <Summary cases={agentCases()} />
      <P>
        Fewer tokens is a smaller bill. The time comes from asking together: every item waits for the model at the
        same time, instead of in a queue. The answers aren&apos;t always the same: each case below says how many every
        way got right.
      </P>
      <H2>The ways compared</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <tbody>
            {agentWays().map(({ way, what }) => (
              <tr key={way} className="border-b border-line align-top">
                <td className="py-2 pr-4 font-semibold whitespace-nowrap text-ink">{way}</td>
                <td className="py-2 text-muted">{code(what)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Every way gets the same inputs and the same expected answers, and uses the same models. A proxy in front of the
        API counts the tokens of every call, for every way the same.
      </P>
      {agentCases().map((c) => (
        <section key={c.name} aria-label={c.title} className="flex flex-col gap-3">
          <H2>{c.title}</H2>
          <P>{c.about}.</P>
          <Results c={c} />
          <TokenBars c={c} />
          <Findings c={c} />
          {c.notes.map((n) => (
            <P key={n}>{code(n)}</P>
          ))}
          {caseCode[c.name] && (
            <>
              <P>{caseCode[c.name].about}</P>
              <CodeBlock code={agentFile(`kavach/src/${caseCode[c.name].file}`)} file={caseCode[c.name].file} />
            </>
          )}
          {c.name === "Large" && (
            <>
              <P>
                The requests that try to talk their way past the policy, and the ones too big to refund without a person,
                from the case&apos;s own data:
              </P>
              <TrickyOrders />
              <P>
                The 900 USD order is a real damage claim, which the policy alone would refund. In Kavach the agent&apos;s{" "}
                <C>limit Payments.refund {"{ amount <= 500 }"}</C> refuses the payment, so a person gets it: the model
                never needs to know the rule. Here is the agent:
              </P>
              <CodeBlock code={agentFile("kavach/src/refunds.kav")} file="refunds.kav" />
              <P>
                The plain-prompt way must write the same rule beside the code that pays, and check the JSON by hand:
              </P>
              <CodeBlock code={pythonFunction("python/refunds.py", "plain")} file="refunds.py: plain prompt" lang="text" />
              <P>
                The agentic loop lets the model pick the tool, so only the tool&apos;s own check stands between a note
                and a payment, and every step sends the tools and the whole conversation again:
              </P>
              <CodeBlock code={pythonFunction("python/refunds.py", "agentic")} file="refunds.py: agentic tool loop" lang="text" />
            </>
          )}
        </section>
      ))}
      <H2>What the usual ways risk</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">A plain prompt</strong> gives text. A missing field or a misspelled choice reaches the program unless someone writes the check, every time.</li>
        <li><strong className="text-ink">A rule like &quot;over 500 needs a person&quot;</strong> must be repeated at every place that pays. In Kavach it is one <C>limit</C>, checked on every call: see <Link href="/docs/agents/agents/limit/">limit</Link>.</li>
        <li><strong className="text-ink">An agentic tool loop</strong> trusts the model to choose the action; an injected note is stopped only if each tool checks for itself, and it pays for that in tokens.</li>
      </ul>
      <Note>
        Measuring found two things Kavach now handles: a provider turns away too many calls at once, so a busy or dropped
        call is sent again after a growing wait (see <Link href="/docs/agents/ask/when-the-ai-fails/">When the AI fails</Link>),
        and asking for the type in the prompt costs a third of the tokens of structured output (see{" "}
        <Link href="/docs/agents/ask/">ask&lt;T&gt;</Link>).
      </Note>
      <H2>Run it yourself</H2>
      <P>It calls the real model, so it costs tokens. The key is read from the environment and never written down.</P>
      <Terminal commands session={agentRunCommand()} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="benchmarks/agents/README.md" label="the benchmark, with every case" />
        <SrcRef file="benchmarks/agents/results/2026-11-04.json" label="every answer of the run" />
        <SrcRef file="docs/decisions/0057-ask-puts-the-type-in-the-prompt.md" label="decision 0057" />
      </div>
    </>
  ),
};
