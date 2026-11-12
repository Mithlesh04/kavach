import Link from "next/link";
import { C } from "@/components/inline-code";
import { winsAgainstCpp } from "@/components/ratio";
import { SmartBox } from "@/components/smart-box";
import { SrcRef } from "@/components/src-ref";
import project from "@/data/project.json";
import versions from "@/data/versions.json";
import { smart, type SmartGroup } from "@/content/smart";
import type { LessonDoc } from "@/lib/chapter";
import { findNode } from "@/lib/nav";
import { hasContent } from "@/lib/site";
import { Ex, H2, Note, P } from "./parts";

// ---------- Kavach Home ----------

function KindOfLanguage() {
  const { wins, total } = winsAgainstCpp();
  const rows: [string, React.ReactNode, React.ReactNode][] = [
    [
      "Compiled or interpreted?",
      <>
        <strong className="text-ink">Compiled.</strong> <C>kavach build</C> turns your program into machine code:
        one executable file, which needs no Kavach, interpreter or virtual machine to run. While you learn, <C>kavach run</C> starts a
        program at once through a built-in interpreter, with the same results.
      </>,
      <SrcRef key="b" file="docs/decisions/0014-development-interpreter.md" label="decision 0014" />,
    ],
    [
      "Is it a scripting language?",
      <>
        No, but it is as quick to try as one. A file can be a single line, <C>print(&quot;Hello&quot;)</C>,
        with no <C>main</C>, no project and no imports. The difference: Kavach checks the whole program before
        it runs, and the same file compiles to fast native code.
      </>,
      <SrcRef key="s" file="docs/spec/10-modules-and-packages/09-running-code-quickly.md" line={3} symbol="scripting language" label="spec 10" />,
    ],
    [
      "How are types handled?",
      <>
        <strong className="text-ink">Statically typed.</strong> Every value&apos;s type is known and checked before
        the program runs, so a wrong type is a message, not a crash. You rarely write types yourself: the
        compiler works them out.
      </>,
      <SrcRef key="t" file="docs/decisions/0002-static-types-with-inference.md" label="decision 0002" />,
    ],
    [
      "Who manages memory?",
      <>
        Kavach does, <strong className="text-ink">without a garbage collector</strong>: a value is freed the
        moment nothing uses it, so there are no pauses.
      </>,
      <SrcRef key="m" file="docs/decisions/0003-memory-model.md" label="decision 0003" />,
    ],
    [
      "What is it for?",
      <>
        It is <strong className="text-ink">general-purpose</strong>: command-line tools, servers, data
        processing, AI agents, GPU work, and libraries other languages call.
      </>,
      <SrcRef key="v" file="docs/vision/vision.md" label="vision" />,
    ],
    [
      "What is it built with?",
      <>
        The compiler and runtime are written in <strong className="text-ink">Rust</strong>, a memory-safe language,
        with <C>unsafe</C> Rust allowed in one place only: the small runtime library linked into programs. Machine
        code comes from <strong className="text-ink">LLVM</strong>, or from Cranelift for quick runs. Much of the
        standard library is written in <strong className="text-ink">Kavach itself</strong>: HTTP, gRPC, logging and
        random numbers among it.
      </>,
      <SrcRef key="r" file="docs/architecture/overview.md" label="how it is built" />,
    ],
    [
      "How fast is it?",
      <>
        Faster than C++ in {wins} of our {total} published benchmarks, measured on one machine, with the losses
        published too.
      </>,
      <SrcRef key="n" file="benchmarks/README.md" label="benchmarks" />,
    ],
  ];
  return (
    <dl className="m-0 grid grid-cols-1 gap-3">
      {rows.map(([q, a, proof]) => (
        <div key={q} className="rounded-xl border border-line bg-panel px-5 py-4">
          <dt className="mb-1 text-[15.5px] font-extrabold text-ink">{q}</dt>
          <dd className="m-0 text-[15.5px] leading-relaxed text-muted">{a}</dd>
          <dd className="m-0 mt-2.5">{proof}</dd>
        </div>
      ))}
    </dl>
  );
}

export const homeDoc: LessonDoc = {
  lead: "Kavach (कवच) is a compiled, statically typed, general-purpose programming language: as quick to try as a script, and checked and fast like a systems language.",
  simple: <>A programming language is a way to write instructions a computer can follow. Kavach reads all of your instructions and checks them before any of them run, like a friend proofreading a letter before you post it.</>,
  minutes: 4,
  body: (
    <>
      <H2>What kind of language is Kavach?</H2>
      <KindOfLanguage />
      <H2>A whole program</H2>
      <P>This file is a complete Kavach program. Save it as <C>hello.kav</C> and run <C>kavach run hello.kav</C>:</P>
      <Ex name="start/script" />
      <H2>How this tutorial works</H2>
      <P>
        Every page teaches <strong className="text-ink">one idea</strong>. When an idea has several forms, each
        form gets a page of its own under it: the For Loop page has &quot;Over a list&quot;, &quot;Over a
        range&quot; and more. Every example runs, and the output under it is what the real compiler printed when
        this site was built.
      </P>
      <P>
        A page then has a <strong className="text-ink">Try it yourself</strong> task, a box saying{" "}
        <strong className="text-ink">what Kavach does for you</strong> there, and a closed{" "}
        <strong className="text-ink">Go deeper</strong> part with the why, for readers who want it.
      </P>
      <H2>Where to start</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Never written a program? Start with <Link href="/docs/start/your-first-program/">Your First Program</Link> and read the pages in order.</li>
        <li>You know another language? Read <Link href="/docs/start/what-kavach-does-for-you/">What Kavach Does for You</Link> and <Link href="/docs/start/classic-mistakes-fixed/">Classic Mistakes, Fixed</Link>, then skim Basics.</li>
        <li>Looking something up? Use search: it knows other languages&apos; words too, so &quot;array&quot; finds lists.</li>
      </ul>
      <H2>Who makes Kavach</H2>
      <P>
        Kavach (<span lang="hi">{project.nameHindi}</span>) is created and led by{" "}
        <a href={project.lead.url}>{project.lead.name}</a>, who also writes this course. It is open source: the
        language, its compiler and this site live in <a href={versions.repo}>one repository</a>, and every page links
        to the code and measurements behind it.
      </P>
    </>
  ),
};

// ---------- What Kavach Does for You ----------

const groups: SmartGroup[] = ["Fewer mistakes", "Numbers and types", "Clear messages", "Fast without effort", "Safe by default"];

function Catalog() {
  return (
    <>
      {groups.map((g) => (
        <section key={g} aria-label={g}>
          <SmartBox
            title={g}
            things={smart.filter((s) => s.group === g)}
            extra={(s) => {
              const page = findNode(s.where);
              if (!page || !hasContent(page.key)) return null;
              const name = page.sub ? `${page.page.title}: ${page.title}` : page.title;
              return (
                <Link href={page.href} className="text-sm font-bold">
                  Learn it: {name} →
                </Link>
              );
            }}
          />
        </section>
      ))}
    </>
  );
}

export const smartDoc: LessonDoc = {
  lead: "The things Kavach does without being asked: fewer mistakes, exact numbers, clear messages, speed and safety. Each one links to its proof.",
  simple: <>Much of Kavach&apos;s help is invisible: it happens while you write and run your program, like a careful assistant who checks your work without being asked. This page gathers all of that help in one place.</>,
  minutes: 8,
  body: (
    <>
      <P>
        Much of what makes Kavach pleasant never shows in your code: the compiler and the runtime do it for you.
        Here is all of it in one place. Each item also appears on the page that teaches it.
      </P>
      <Catalog />
    </>
  ),
};

// ---------- Your First Program ----------

export const firstProgramDoc: LessonDoc = {
  lead: "Write a file, run it, and turn it into a program anyone can start.",
  simple: <>You write your instructions in a file and run them at once with <C>kavach run</C>. <C>kavach build</C> is like baking the cake instead of handing over the recipe: the result works on a machine of the same kind, with no Kavach needed.</>,
  minutes: 5,
  before: ["start/install"],
  body: (
    <>
      <P>
        Every Kavach program starts at <C>fn main()</C>. The lines between its braces run from top to bottom.
      </P>
      <Ex name="start/hello" />
      <H2>Run it</H2>
      <P>
        Save the code as <C>hello.kav</C>, open a terminal in the same folder, and type{" "}
        <C>kavach run hello.kav</C>. The two lines above appear.
      </P>
      <H2>Make an executable</H2>
      <P>
        <C>kavach build hello.kav</C> compiles the same file to machine code and writes <C>hello</C>{" "}
        (<C>hello.exe</C> on Windows). That file is the whole program: give it to someone with the same kind of
        machine and it runs there without Kavach.
      </P>
      <Note>
        <C>kavach run</C> while you write, <C>kavach build</C> when you ship. Both give the same output.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Change the program so it prints your own name on the first line.</>,
    answer: <P>Replace the text inside the first <C>print(…)</C>, keeping the quotes: <C>print(&quot;Asha&quot;)</C>.</P>,
  },
  quiz: [
    {
      type: "choice",
      prompt: <>You wrote hello.kav. Which command makes a file you can give to someone without Kavach?</>,
      options: ["kavach build hello.kav", "kavach run hello.kav", "kavach check hello.kav"],
      answer: 0,
      explain: "build compiles to machine code: one executable file. run starts it now; check only looks for errors.",
    },
  ],
};
