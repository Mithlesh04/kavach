import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// The motivation, from docs/vision/vision.md: what changed, and what
// Kavach does that existing languages weren't built to do. Each row links
// to the page that proves it with a checked sample.

const answers: [problem: string, elsewhere: string, kavach: React.ReactNode, page: string][] = [
  [
    "What can this code touch?",
    "Read its whole body, and every library it calls.",
    <>The first line says: <C>uses Fs.read</C>. The compiler checks it.</>,
    "/docs/agents/capabilities/",
  ],
  [
    "Can this AI agent spend too much, or act without a person's yes?",
    "A prompt asks the model nicely; glue code checks, if someone remembered to write it.",
    <><C>limit</C> and <C>approve</C> are checked on every call, outside anything the model can influence.</>,
    "/docs/agents/agents/",
  ],
  [
    "Will the model's answer have the right shape?",
    "Parse its text and hope.",
    <><C>ask&lt;T&gt;</C> gives a value of type <C>T</C> that passed the checks, or an error.</>,
    "/docs/agents/ask/",
  ],
  [
    "Can a card number leak into a log or a prompt?",
    "Code review, and tools that find the leak afterwards.",
    <>A <C>secret</C> value that reaches a log, a prompt or JSON doesn&apos;t compile.</>,
    "/docs/agents/secrets/",
  ],
  [
    "What if the machine dies halfway through a payment?",
    "Build your own retries, and hope nothing runs twice.",
    <>A journal: after a restart the work finishes, and nothing is paid twice.</>,
    "/docs/agents/durable-agents/",
  ],
  [
    "Is this code an AI wrote correct?",
    "Read all 300 lines.",
    <>Read the signature: what it takes, gives, may fail with, may touch, and promises.</>,
    "/docs/functions/signatures/",
  ],
];

export const whyDoc: LessonDoc = {
  lead: "There are already hundreds of languages, and many make life easier. Kavach exists for something they weren't built for: AI writes much of today's code, and AI agents now take real actions.",
  simple: <>AI now writes much code, and AI agents do real things, such as refunding orders. Kavach puts what code may do into the code itself, like a spending limit on a bank card that is checked on every payment.</>,
  minutes: 5,
  body: (
    <>
      <H2>A fair question</H2>
      <P>
        Python is quick for scripts, JavaScript runs the web, Go makes servers simple, and Rust and C++ are fast and
        precise. They are good languages. If one of them serves you well, keep using it: Kavach doesn&apos;t try to
        be a better Python, Rust or Go for everything.
      </P>
      <P>
        But those languages were designed for people writing code by hand. Nothing in a Python or JavaScript
        function tells you whether it can touch the network, spend money, or leak a customer&apos;s card number.
      </P>
      <H2>What changed</H2>
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">AI writes much of the code, and people review it.</strong> Reviewing 300 generated lines is slow, and people miss things.</li>
        <li><strong className="text-ink">AI agents take real actions.</strong> They refund orders, change records, send emails and deploy code.</li>
        <li><strong className="text-ink">Companies don&apos;t trust those agents yet.</strong> Today&apos;s guardrails are spread over prompts, glue code and checks added later. None of them is a hard guarantee.</li>
      </ol>
      <H2>What Kavach does about it</H2>
      <P>In Kavach, what code is allowed to do is part of the code: the compiler checks it, and the runtime enforces it.</P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">The question</th>
              <th className="py-2 pr-4 font-bold">Usually</th>
              <th className="py-2 font-bold">In Kavach</th>
            </tr>
          </thead>
          <tbody>
            {answers.map(([q, elsewhere, kavach, page]) => (
              <tr key={q} className="border-b border-line align-top">
                <td className="py-2 pr-4 font-semibold text-ink">{q}</td>
                <td className="py-2 pr-4 text-muted">{elsewhere}</td>
                <td className="py-2">
                  {kavach} <Link href={page} className="whitespace-nowrap">See it →</Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>An AI shopping assistant, for example. Elsewhere its budget would live in a prompt; here it lives in the code:</P>
      <Ex name="agents/uc-shopping" />
      <H2>And an everyday language too</H2>
      <P>Speed and ease are not the reason to switch, but you don&apos;t give them up:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Fast:</strong> compiled to native code with no garbage collector, measured against C++ and Python on the <Link href="/docs/benchmarks/">Benchmarks</Link> page, with the steps to check every number yourself.</li>
        <li><strong className="text-ink">Easy to learn:</strong> a small language, types worked out for you, and errors that say what to write instead.</li>
        <li><strong className="text-ink">Fewer traps:</strong> <C>0.1 + 0.2</C> is <C>0.3</C>, and a number too big stops with a message instead of turning wrong (<Link href="/docs/start/classic-mistakes-fixed/">Classic Mistakes, Fixed</Link>).</li>
        <li><strong className="text-ink">Script or program:</strong> run a file with <C>kavach run</C> like a script, or build one executable to ship.</li>
        <li><strong className="text-ink">Works with what you have:</strong> Kavach calls C libraries, and Python, Node.js or Go can call a Kavach library, so you can adopt it one part at a time. Calling Python and JavaScript packages directly comes later.</li>
      </ul>
      <H2>The name</H2>
      <P>
        <strong className="text-ink">Kavach (कवच)</strong> means <em>armour</em> in Hindi: the protection a warrior
        wears. The language is armour around code that acts in the world, so a mistake, whether a person&apos;s or an
        AI&apos;s, is stopped before it does harm.
      </P>
      <Note>
        Kavach is young: it is in its first phase, before its first public release. Learn it, try it and build
        tools with it now; see <Link href="/docs/start/who-kavach-is-for/">Who Kavach Is For</Link> for what comes later.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/vision/vision.md" label="the vision" />
        <SrcRef file="docs/vision/developer-needs.md" label="what developers ask for" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>What is the main reason Kavach exists?</>,
      options: [
        "So that what code may do is part of the code, checked by the compiler and the runtime, which matters when AI writes code and agents act",
        "To replace Python, JavaScript and Go for everything",
        "To be the fastest language at any cost",
      ],
      answer: 0,
      explain: "Speed and ease are table stakes. The reason is guarantees about what code, and the agents it runs, can do.",
    },
  ],
};
