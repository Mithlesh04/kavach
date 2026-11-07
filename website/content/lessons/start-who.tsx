import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { findNode } from "@/lib/nav";
import { hasContent } from "@/lib/site";
import { H2, Note, P } from "./parts";

/** A link to a course page when it is written; plain text until then, so no link leads nowhere. */
function PageLink({ to, children }: { to: string; children: React.ReactNode }) {
  const node = findNode(to);
  if (!node) throw new Error(`What You Can Build: no course page ${to}`);
  return hasContent(to) ? <Link href={node.href}>{children}</Link> : <span>{children}</span>;
}

// ---------- Who Kavach Is For ----------

const audiences: [string, React.ReactNode][] = [
  [
    "Teams putting AI agents into production",
    <>Fintech, e-commerce, SaaS and internal tools: Kavach lets you prove what an agent can and can&apos;t do, with limits, approvals and an audit log written into the code.</>,
  ],
  [
    "Backend and systems developers",
    <>Native speed and memory safety in a language that is quick to learn: servers, command-line tools, data processing, and libraries other languages call.</>,
  ],
  [
    "People learning to program",
    <>One idea per page, every example runnable, and a compiler whose messages say what to write instead. This course starts from what a program is.</>,
  ],
  [
    "Developers who review code an AI wrote",
    <>A function&apos;s first lines say what it takes, gives, may fail with and may touch, so you can review the contract instead of every line.</>,
  ],
];

export const whoDoc: LessonDoc = {
  lead: "Kavach is for teams that need AI agents they can trust, developers who want speed without memory bugs, and anyone learning to program.",
  simple: <>Never programmed before? Kavach is for you too: this course starts from what a program is, teaches one idea per page, and every example runs.</>,
  minutes: 3,
  body: (
    <>
      <div className="grid grid-cols-[repeat(auto-fit,minmax(260px,1fr))] gap-3">
        {audiences.map(([who, why]) => (
          <div key={who} className="rounded-xl border border-line bg-panel px-5 py-4">
            <p className="m-0 mb-1 text-base font-extrabold text-ink">{who}</p>
            <p className="m-0 text-[15px] leading-relaxed text-muted">{why}</p>
          </div>
        ))}
      </div>
      <H2>Later, and not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Later:</strong> user interfaces, GPU graphics and microcontrollers come after the core platform.</li>
        <li><strong className="text-ink">Through Python:</strong> data-science notebooks; Kavach calls Python libraries instead of replacing them.</li>
        <li>
          <strong className="text-ink">Not a no-code tool:</strong> people who don&apos;t program will talk to AI in their own
          language. Kavach is the precise layer the AI writes and the compiler checks.
        </li>
      </ul>
      <Note>
        Kavach is young: it is in its first phase, before its first public release. It is ready to learn, try and build
        tools with; wait for a stable release before betting a production system on it.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/vision/vision.md" label="the vision: who it is for" />
        <SrcRef file="docs/vision/roadmap.md" label="the roadmap" />
      </div>
    </>
  ),
};

// ---------- What You Can Build ----------

type Idea = { title: string; what: string; uses: [label: string, page: string][] };

const ideas: [level: string, color: string, list: Idea[]][] = [
  ["Your first projects", "text-ok", [
    { title: "Tip splitter", what: "Ask for the bill and the number of people, print each share to the paisa.", uses: [["User Input", "basics/user-input"], ["Decimal", "basics/data-types/decimal"]] },
    { title: "Number guessing game", what: "The program picks a number; the player guesses until right, with \"higher\" and \"lower\" hints.", uses: [["While Loop", "basics/while-loop"], ["If...Else", "basics/if-else"], ["Random Numbers", "standard-library/random-numbers"]] },
    { title: "Word counter", what: "Count every word of a text and print the ten most common.", uses: [["Maps", "collections/maps"], ["Strings", "basics/strings"], ["Sorting", "collections/lists/sorting"]] },
    { title: "Quiz in the terminal", what: "Questions in a list of records, answers checked, a score at the end.", uses: [["Records", "types/records"], ["For Loop", "basics/for-loop"], ["User Input", "basics/user-input"]] },
  ]],
  ["Growing programs", "text-accent-text", [
    { title: "Expense report", what: "Read a JSON file of expenses, group them by month, print a colored table.", uses: [["JSON", "standard-library/json"], ["Files", "standard-library/files"], ["Maps", "collections/maps"]] },
    { title: "To-do list that remembers", what: "Add, finish and list tasks; save them to a file between runs.", uses: [["Files", "standard-library/files"], ["Errors", "errors/errors"], ["Modules", "tools/modules"]] },
    { title: "Weather in your terminal", what: "Call a public JSON weather API and print today's forecast.", uses: [["HTTP", "networks/http"], ["JSON", "standard-library/json"], ["Optionals", "types/optionals"]] },
    { title: "A small JSON API", what: "Serve your own data over HTTP, with types that check every request.", uses: [["HTTP", "networks/http"], ["Records", "types/records"], ["Testing", "errors/testing"]] },
  ]],
  ["Expert projects", "text-ch-c", [
    { title: "A refund agent", what: "An AI agent that refunds orders, asks a person above a limit, logs everything and resumes after a crash.", uses: [["Agents", "agents/agents"], ["Capabilities", "agents/capabilities"], ["Durable Agents", "agents/durable-agents"]] },
    { title: "A gRPC service", what: "A typed service other languages call, with streaming and TLS.", uses: [["gRPC", "networks/grpc"], ["Tasks", "concurrency/tasks"]] },
    { title: "A data cruncher on every core", what: "Sum, filter and sort hundreds of millions of items, spread over all cores.", uses: [["Loops on All Cores", "speed/loops-on-all-cores"], ["Channels", "concurrency/channels"]] },
    { title: "A GPU simulation", what: "Millions of particles stepped on the GPU, results identical to the CPU.", uses: [["GPU", "speed/gpu"], ["Data Types", "basics/data-types"]] },
    { title: "A library for Python", what: "Write the fast part in Kavach and call it from Python, Node.js or Go.", uses: [["A Library for Any Language", "other-languages/a-library-for-any-language"], ["Calling C", "other-languages/calling-c"]] },
  ]],
];

export const buildDoc: LessonDoc = {
  lead: "Project ideas from your first afternoon to expert work, each with the pages that teach what it needs.",
  simple: <>Learning a language is like learning to cook: you learn fastest by making a dish you want to eat. Pick a project at your level, and open the lessons listed under it.</>,
  minutes: 4,
  body: (
    <>
      <P>
        Every idea here uses only what Kavach does today. Pick one at your level, open the pages under it, and build it
        piece by piece; the best way to learn a language is to make something you want.
      </P>
      {ideas.map(([level, color, list]) => (
        <section key={level} aria-label={level} className="flex flex-col gap-3">
          <h2 className={`m-0 mt-3 text-[22px] font-extrabold tracking-tight ${color}`}>{level}</h2>
          <div className="grid grid-cols-[repeat(auto-fit,minmax(260px,1fr))] gap-3">
            {list.map((idea) => (
              <div key={idea.title} className="flex flex-col gap-1.5 rounded-xl border border-line bg-panel px-5 py-4">
                <p className="m-0 text-base font-extrabold text-ink">{idea.title}</p>
                <p className="m-0 text-[15px] leading-relaxed text-muted">{idea.what}</p>
                <p className="m-0 mt-auto pt-1 text-[13px] text-soft">
                  Uses:{" "}
                  {idea.uses.map(([label, page], i) => (
                    <span key={page}>
                      {i > 0 && ", "}
                      <PageLink to={page}>{label}</PageLink>
                    </span>
                  ))}
                </p>
              </div>
            ))}
          </div>
        </section>
      ))}
      <Note>
        Built something? The <C>examples/</C> folder of the repository has finished programs to read and run.
      </Note>
    </>
  ),
};
