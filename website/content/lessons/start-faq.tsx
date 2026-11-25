import Link from "next/link";
import { Fragment, type ReactNode } from "react";
import { withCode } from "@/components/inline-code";
import { winsAgainstCpp } from "@/components/ratio";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { SITE_URL, visibleNodes } from "@/lib/site";
import { fileTables } from "@/lib/spec-tables";
import project from "@/data/project.json";
import versions from "@/data/versions.json";
import { P } from "./parts";

// ---------- FAQ ----------
//
// What developers ask before they try Kavach. Each answer is written once,
// as text with `code` and [links](…): the page shows it, and the page's
// FAQPage data (for search engines and AI crawlers) reads the same words.
// Numbers are read from their sources at build time, never typed: the
// AI-writability run (benchmarks/ai-writes/README.md) and the published
// benchmarks. A link to a course page that doesn't exist fails the build.

const AI_README = "benchmarks/ai-writes/README.md";

/** The lowest and highest pass rate in a column of the AI run's Pass@1 table, as "88% to 97%". */
function passRange(heading: string, column: string): string {
  const t = fileTables(AI_README).find((x) => x.heading === heading);
  const at = t?.header.indexOf(column) ?? -1;
  if (!t || at < 0) throw new Error(`${AI_README}: no column ${column} under ## ${heading}`);
  const shares = t.rows.map((r) => Number(r[at].match(/^(\d+)%/)?.[1]));
  if (shares.some((n) => Number.isNaN(n))) throw new Error(`${AI_README}: ${column} has a cell that isn't a share`);
  const [low, high] = [Math.min(...shares), Math.max(...shares)];
  return low === high ? `${low}%` : `${low}% to ${high}%`;
}

const ai = {
  zero: passRange("Pass@1", "zero"),
  docs: passRange("Pass@1", "docs"),
  tools: passRange("Pass@1", "tools"),
  python: passRange("Against Python", "Python, zero"),
};
const { wins, total, left } = winsAgainstCpp();
const ROADMAP = `${versions.repo}/blob/main/docs/vision/roadmap.md`;

type Question = { q: string; a: string[] };
type Group = { title: string; questions: Question[] };

const groups: Group[] = [
  {
    title: "Why a new language",
    questions: [
      {
        q: "Why build a new language at all?",
        a: [
          "Because of one problem that kept coming up. More and more code is written by AI, and more of it runs as agents that do real things: refund an order, email a customer, change a record. In Python, Go or TypeScript, nothing in a function tells you what it can touch. You find out by reading every line, or after something goes wrong.",
          "In Kavach that is part of the code. A function says what it uses (`uses Fs`, `uses Net`), the compiler checks it, and an agent's limits sit right next to it. That's very hard to add to an existing language without breaking the code already written in it.",
        ],
      },
      {
        q: "Couldn't this just be a library for Python or TypeScript?",
        a: [
          "Partly, and guardrail libraries for agents do exist. The difference is where the check happens. A library checks what you remember to send through it. In Kavach the compiler refuses a function that reaches the network without saying so, and that includes code you didn't write yourself: a package, or something an AI generated.",
          "Limits, approvals and the audit log are run by the runtime, so a missing line of glue code can't switch them off. [What Kavach Does for You](/docs/start/what-kavach-does-for-you/) lists each check, with the test that proves it.",
        ],
      },
      {
        q: "Who is it for?",
        a: [
          "Mainly teams putting AI agents into production, and backend developers who want native speed without memory bugs. It also works as a first language: the course starts from what a program is.",
          "It isn't trying to replace notebooks for data science, and it isn't a no-code tool. More on [who Kavach is for](/docs/start/who-kavach-is-for/).",
        ],
      },
    ],
  },
  {
    title: "When AI writes the code",
    questions: [
      {
        q: "If AI writes most of the code, why learn Kavach?",
        a: [
          "Because someone still decides what the code is allowed to do, and checks that it does only that. That job doesn't go away when an AI types the lines. It gets bigger.",
          "Kavach is built for that job. You write an agent's limits and approvals yourself, and a function's first line says what it takes, what it gives back, how it can fail and what it can touch. So reviewing AI code is mostly reading signatures and contracts, not every line. Reading Kavach well is the skill, and it's quick to pick up.",
        ],
      },
      {
        q: "AI models weren't trained on Kavach. Can they write it?",
        a: [
          `Not on their own, and we measured it. Given only the task, Claude models wrote a working program ${ai.zero} of the time. Given Kavach's one-page guide for AI tools, [llms.txt](/llms.txt), that went up to ${ai.docs}. With Kavach's own tools through [kavach mcp](/docs/tools/tools-for-ai/mcp/), so the model can check and run its code, it reached ${ai.tools}. The same models got ${ai.python} on the same tasks in Python.`,
          "The language helps: the grammar is small and regular, and every compiler error says what to write instead, which is what a model needs when it fixes its own code. The whole run, failures included, is on [How Well AI Writes Kavach](/docs/tools/how-well-ai-writes-kavach/).",
        ],
      },
      {
        q: "Which AI assistants and editors work with it?",
        a: [
          "Any assistant that can read a web page can read [llms.txt](/llms.txt). Assistants that speak MCP, such as Claude Code or Cursor, can run `kavach mcp` to check, run and test code, and look things up with `kavach outline` and `kavach query` instead of reading whole files, which saves tokens.",
          "For editors there is a VS Code extension in the repository, with highlighting, and errors as you type from `kavach lsp`. See [Tools for AI](/docs/tools/tools-for-ai/).",
        ],
      },
    ],
  },
  {
    title: "Trust and production",
    questions: [
      {
        q: "Is Kavach ready for production?",
        a: [
          "Not yet, and it's better to say that plainly. Kavach is before its first public release: the core language (Phase 1) is done, and agents (Phase 2) are under way. The compiler builds native programs on Windows and Linux, agents work end to end, and every conformance test passes in the interpreter and in native code. But the language can still change, and there is no stability promise until version 1.0.",
          `Use it to learn, to try ideas and for internal tools. Wait for a stable release before you put a production system on it. The [roadmap](${ROADMAP}) says what each phase has to prove before the next one starts.`,
        ],
      },
      {
        q: "Why would a company trust a new language?",
        a: [
          "It shouldn't take it on faith, so Kavach is built to be checked. There is a written spec, and its rules are checked by conformance tests that run in the interpreter and in native code. Every example on this site runs against the real compiler when the site is built, and claims link to the file and line that back them.",
          "Adopting it doesn't need trust up front either. An agent can start in `observe` mode, where its limits only report and never block, then move to `suggest`, where a person approves each action, and only then to `enforce`. And you can start with one agent or one module, not a whole codebase. See [Agent Modes](/docs/agents/agent-modes/).",
        ],
      },
      {
        q: "What happens if the project stops?",
        a: [
          "That's a fair worry with any young language. Kavach is open source under the MIT and Apache 2.0 licenses, so the code can't be taken away, and anyone can carry it on.",
          "`kavach build` makes an ordinary native executable, and it runs without Kavach installed. Kavach code calls C and can be built as a library other languages call, so a part written in Kavach can be swapped out later, one part at a time.",
        ],
      },
      {
        q: "Who is building it?",
        a: [
          `Kavach is led by [${project.lead.name}](${project.lead.url}), who also writes this course. It's built in the open on [GitHub](${versions.repo}): every design decision has a written record there, with the options that were turned down and why.`,
        ],
      },
    ],
  },
  {
    title: "Compared with what you use now",
    questions: [
      {
        q: "How is it different from Python, Go, Rust or TypeScript?",
        a: [
          "It compiles to native code with no garbage collector, like Rust or C++, but reads closer to Python or Go. Integers and list indexes are checked, so an overflow or a read past the end stops the program instead of quietly giving a wrong answer. A function that can fail says so in its type, and the caller has to handle it: there are no exceptions.",
          "The real difference is the part none of them has: what code may touch is in its signature, and agents with limits, approvals, an audit log and typed AI calls are part of the language. If you know one of those languages, [Coming From Another Language](/docs/start/coming-from-another-language/) maps what you know onto Kavach.",
        ],
      },
      {
        q: "Is it fast?",
        a: [
          `In the like-for-like benchmarks, where Kavach and C++ run on the same number of cores, Kavach's native code is faster in ${wins} of ${total}. Each one comes with a single command to run it yourself, and the losses are shown next to the wins.`,
          `${left} more aren't counted: Kavach ran on all cores there and C++ on one, which says nothing about which language is faster. Their charts say so, and fair runs of them are being measured. See [Benchmarks](/docs/benchmarks/).`,
        ],
      },
      {
        q: "Do I have to rewrite my code to use it?",
        a: [
          "No. Kavach calls C libraries directly, and a Kavach library can be called from Python, Node.js or Go. So you can write one part in Kavach, the fast part or the risky part, and leave the rest where it is. Calling Python from Kavach is planned, not built yet.",
          "See [Calling C](/docs/other-languages/calling-c/) and [A Library for Any Language](/docs/other-languages/a-library-for-any-language/).",
        ],
      },
      {
        q: "Why would I move from a language that already works?",
        a: [
          "If your language works for you and you don't run AI agents, you probably shouldn't, at least not yet. Kavach is worth it when you need to show what code can and can't do: an agent that handles money or customer data, AI-written code you have to sign off on, or a service where a silent overflow would be a real bug.",
          "Start with one of those, not with a rewrite.",
        ],
      },
    ],
  },
  {
    title: "Getting started",
    questions: [
      {
        q: "How do I try it?",
        a: [
          "The quickest way is the [playground](/playground/), right in your browser. To install it, follow [Install](/docs/start/install/), then write [your first program](/docs/start/your-first-program/).",
          "Today it runs on Windows 10 or 11 and on Linux, on 64-bit Intel or AMD processors. macOS and ARM come later.",
        ],
      },
      {
        q: "Is it free?",
        a: ["Yes. The compiler, the standard library and these docs are open source, under the MIT or the Apache 2.0 license, whichever you prefer."],
      },
      {
        q: "What does the name mean?",
        a: [
          `Kavach (${project.nameHindi}) is the Hindi word for armor. The keywords are English, and you don't need any Hindi to use it. The name is about what it does: it keeps code inside the limits you set.`,
        ],
      },
    ],
  },
];

/** An id from a title, as the course's headings make them. */
const idOf = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");

const FILES = new Set(["/llms.txt", "/playground/", "/docs/benchmarks/"]);

/** An answer's text: `code` as code, [text](href) as a link; a course link must lead to a page. */
function Rich({ text }: { text: string }) {
  // Read when rendered, not when the module loads: the course lists every lesson, this one among them.
  const pages = new Set(visibleNodes().map((n) => n.href));
  const out: ReactNode[] = [];
  let rest = text;
  for (let m = rest.match(/\[([^\]]+)\]\(([^)]+)\)/); m && m.index !== undefined; m = rest.match(/\[([^\]]+)\]\(([^)]+)\)/)) {
    const [whole, label, href] = m;
    if (href.startsWith("/") && !pages.has(href) && !FILES.has(href)) throw new Error(`FAQ: ${href} is not a page of the site`);
    out.push(<Fragment key={out.length}>{withCode(rest.slice(0, m.index))}</Fragment>);
    out.push(
      href.startsWith("/") ? (
        <Link key={out.length} href={href}>{withCode(label)}</Link>
      ) : (
        <a key={out.length} href={href}>{withCode(label)}</a>
      ),
    );
    rest = rest.slice(m.index + whole.length);
  }
  out.push(<Fragment key={out.length}>{withCode(rest)}</Fragment>);
  return <>{out}</>;
}

/** An answer as plain text, for structured data: links keep their text, code loses its marks. */
const plain = (text: string) => text.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1").replace(/`/g, "");

/** The page's questions and answers as schema.org FAQPage data. Built when rendered, as `SITE_URL` comes through the course. */
const structured = () => ({
  "@context": "https://schema.org",
  "@type": "FAQPage",
  url: `${SITE_URL}/docs/start/faq/`,
  mainEntity: groups.flatMap((g) =>
    g.questions.map((x) => ({
      "@type": "Question",
      name: x.q,
      acceptedAnswer: { "@type": "Answer", text: x.a.map(plain).join("\n\n") },
    })),
  ),
});

function Faq() {
  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(structured()).replace(/</g, "\\u003c") }} />
      <nav aria-label="Topics" className="flex flex-wrap gap-2">
        {groups.map((g) => (
          <a
            key={g.title}
            href={`#${idOf(g.title)}`}
            className="rounded-full border border-line bg-panel px-3.5 py-1.5 text-[14px] font-bold text-ink no-underline hover:border-accent"
          >
            {g.title}
          </a>
        ))}
      </nav>
      {groups.map((g) => (
        <section key={g.title} aria-labelledby={idOf(g.title)} className="flex flex-col gap-2.5">
          <h2 id={idOf(g.title)} className="m-0 mt-3 text-[22px] font-extrabold tracking-tight min-[1920px]:text-[25px]">
            {g.title}
          </h2>
          {g.questions.map((x) => (
            <details key={x.q} id={idOf(x.q)} className="group rounded-xl border border-line bg-panel open:border-accent">
              <summary className="flex cursor-pointer list-none items-start justify-between gap-4 px-5 py-4 text-base font-extrabold text-ink [&::-webkit-details-marker]:hidden">
                <span>{x.q}</span>
                <span aria-hidden="true" className="mt-0.5 shrink-0 text-xl leading-none text-soft transition-transform group-open:rotate-45">
                  +
                </span>
              </summary>
              <div className="flex flex-col gap-3 px-5 pb-5 text-[15.5px] leading-relaxed text-muted">
                {x.a.map((p) => (
                  <p key={p} className="m-0 max-w-[72ch]">
                    <Rich text={p} />
                  </p>
                ))}
              </div>
            </details>
          ))}
        </section>
      ))}
    </>
  );
}

export const faqDoc: LessonDoc = {
  lead: "Straight answers to what developers ask first: why a new language, why learn one when AI writes the code, whether AI can write it, and whether to trust it yet.",
  minutes: 6,
  body: (
    <>
      <P>
        The questions people ask most before they try Kavach, answered as plainly as we can, including where Kavach
        isn&apos;t ready yet. Open any question to read its answer.
      </P>
      <Faq />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/vision/vision.md" label="the vision" />
        <SrcRef file="docs/vision/roadmap.md" label="the roadmap" />
        <SrcRef file={AI_README} label="the AI-writability run" />
      </div>
    </>
  ),
};
