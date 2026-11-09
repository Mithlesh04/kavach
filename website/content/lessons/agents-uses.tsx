import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- Agent Use Cases ----------

type UseCase = {
  title: string;
  problem: string;
  guarantee: React.ReactNode;
  sample: string;
  after: React.ReactNode;
};

const cases: UseCase[] = [
  {
    title: "A shopping assistant with a budget",
    problem: "An assistant buys what the user asks for. A confused model, or a tricky product page, must never empty the account.",
    guarantee: <>A <C>limit</C> on each purchase, on the day&apos;s total, and on purchases per hour.</>,
    sample: "agents/uc-shopping",
    after: <>The laptop is over the per-item limit; the tea would take the day past 3,000. Both refusals are errors the program handles, and both are in the audit log.</>,
  },
  {
    title: "A deploy bot that asks before production",
    problem: "Releasing to a test server should be quick; releasing to production needs a person on call to agree.",
    guarantee: <><C>approve Deploy.release when env == &quot;production&quot; by &quot;oncall&quot;</C>.</>,
    sample: "agents/uc-deploy",
    after: <>Staging goes out at once. Production waits for the on-call person; in this run no one answered, so nothing was released.</>,
  },
  {
    title: "A support inbox sorted by AI",
    problem: "Each message goes to the right team, without the model being able to do anything but sort.",
    guarantee: <><C>ask&lt;Triage&gt;</C> can only answer with one of the three teams, and the agent can only assign tickets, at most 200 an hour, with at most 1,000 model calls a day.</>,
    sample: "agents/uc-support.dry",
    after: <>Run with <C>--dry-run</C>: the model and the help desk are never called, and the lines show what would have been. Try a new agent this way before it touches anything real.</>,
  },
  {
    title: "A file agent that stays in its folder",
    problem: "An agent reads documents from an inbox folder. A file name it is given, by a person or a model, must not reach anything else.",
    guarantee: <>The implementation says <C>uses Fs.read(&quot;./inbox&quot;)</C>: it can only read, and only there.</>,
    sample: "agents/uc-files",
    after: <><C>../passwords.txt</C> climbs out of the folder, so the read is refused while the program runs, and the agent reports it.</>,
  },
];

export const useCasesDoc: LessonDoc = {
  lead: "Four agents, each with a real risk and the line of code that removes it: a spending limit, a person who approves, a model that can only sort, a folder that can't be left.",
  simple: <>Think of each one as a safety catch: a spending cap, a manager&apos;s signature, a multiple-choice form, a locked door. Each catch is one line of code, and every program here really runs as shown.</>,
  minutes: 8,
  before: ["agents/agents", "agents/ask"],
  body: (
    <>
      <P>
        Each program here runs as shown, and its output is checked on every build of this site. Copy one and change
        it: the compiler tells you when a change would let the agent do more than it says.
      </P>
      {cases.map((c) => (
        <section key={c.title} aria-label={c.title} className="flex flex-col gap-3">
          <H2>{c.title}</H2>
          <P>{c.problem}</P>
          <P><strong className="text-ink">What holds it:</strong> {c.guarantee}</P>
          <Ex name={c.sample} />
          <P>{c.after}</P>
        </section>
      ))}
      <H2>A refund agent, start to finish</H2>
      <P>
        The repository&apos;s refund bot puts it all together: <C>ask</C> decides, limits cap each refund and the
        day&apos;s total, finance approves large ones, every call is audited, and a crash half way never pays twice.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="examples/05-refund-bot/src/main.kav" label="the refund bot example" />
      </div>
      <H2>More ideas</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An expense checker: <C>ask</C> reads each receipt into a typed record, and payouts above a limit go to a manager.</li>
        <li>A newsletter sender: <C>count &lt;= 5_000 per day</C> on <C>Email.send</C>, started in <C>observe</C> mode for its first week.</li>
        <li>A database clean-up agent: it may archive rows, never delete them, because only <C>Db.archive</C> is in its <C>can</C>.</li>
      </ul>
      <Note>
        Every one of these keeps its promise for the same reason: the agent touches the world only through
        capabilities, and those are checked by the compiler and the runtime, not by the model. See{" "}
        <Link href="/docs/start/what-you-can-build/">What You Can Build</Link> for projects of every level.
      </Note>
    </>
  ),
};
