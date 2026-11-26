import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, Files, H2, P } from "./parts";

// ---------- Concepts: the choices for scale ----------
//
// Why Kavach keeps big codebases, AI tools and agents in hand: files kept
// small by the compiler, a language and tools built to spend few tokens,
// and agents whose powers are written in the code.

const ul = "m-0 flex list-disc flex-col gap-1.5 pl-5";

export const conceptSmallFilesDoc: LessonDoc = {
  lead: "Kavach holds every file to 500 lines and every function to 80 by default, and checks a project's architecture in kavach.toml, so a codebase can't decay slowly while nobody looks.",
  simple: <>A book is easier to read in short chapters. Kavach keeps code files short in the same way, and stops a file from growing past a limit, so a person or an AI can always read one in a sitting.</>,
  minutes: 5,
  before: ["tools/small-files"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        Large codebases decay in ways everyone knows: files grow to thousands of lines, modules depend on what they
        shouldn&apos;t, and a shared library breaks its users by accident. Style guides and reviews try to stop it,
        but they depend on people remembering, and they don&apos;t hold across many teams.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Leave it to linters, which a project may or may not run, and to review.</li>
        <li>Check architecture with separate tools, written in another language and run by another job.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        The limits are the compiler&apos;s, set in <C>kavach.toml</C>: 500 lines a file is an error, 80 lines a
        function a warning. Here a project sets a small limit, so the sample stays short:
      </P>
      <Files name="projects/limits.fails" first={["kavach.toml", "src/main.kav"]} outcome="refused" />
      <ul className={ul}>
        <li>Which module may use which is written under <C>[architecture]</C>, and a dependency cycle is an error.</li>
        <li><C>kavach api check</C> finds breaking changes before a release, including ones other languages miss: a new capability, a new error, a stronger <C>require</C>.</li>
        <li>The Kavach repository holds itself to the same 500 lines.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>Sometimes a file must be split that you would have kept whole. Exceptions are possible, and listed in one place, so they are visible.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0009-size-limits-and-enforced-architecture.md" label="decision 0009 · size limits" />
        <SrcRef file="docs/spec/12-maintainability.md" label="spec 12 · maintainability" />
      </div>
    </>
  ),
};

export const conceptAiDoc: LessonDoc = {
  lead: "Kavach is built for AI to write and read: signatures say everything a caller needs, and the tools answer questions about code exactly, so an assistant reads a few lines instead of whole files.",
  simple: <>AI assistants pay for every word they read. Most of it goes on searching and reading files to change a few lines. Kavach&apos;s tools hand them exactly what they need, the way a good index saves you reading a whole book.</>,
  minutes: 5,
  before: ["tools/the-kavach-command"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        AI writes a growing share of code, and every token it reads or writes costs time and money. Watching
        assistants work, most tokens go on finding and reading code: a text search, then whole files, to change a
        few lines; then repeating unchanged code to place an edit, and retrying after unclear errors.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>No mainstream language is designed for it. Editors and assistants search text and read files.</li>
        <li>Language servers answer some questions, through a protocol made for editors rather than for assistants.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        A Kavach signature carries types, errors, capabilities, contracts and examples, so it says how to use a
        function without its body. <C>kavach outline</C> shows a file as its signatures:
      </P>
      <Files name="tools/ai-outline" />
      <ul className={ul}>
        <li><C>kavach query</C> answers questions exactly: callers, users of a capability, a value&apos;s type. <C>kavach context</C> gives a name&apos;s code and what it uses, within a token budget.</li>
        <li>Not built yet: <C>kavach edit</C> and <C>rename</C>, to change declarations by name, checked before saving.</li>
        <li>Errors are short, carry codes, and come with fixes a tool can apply.</li>
        <li><C>llms.txt</C> and <C>kavach mcp</C>, a Model Context Protocol server, give any assistant all of it.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>More tools to build and keep right. Most reuse the compiler&apos;s own resolver and checker, so they stay in step.</li>
        <li>The goal of 90% fewer tokens may not hold for every task. The benchmark reports where it doesn&apos;t.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0015-token-efficient-ai-tooling.md" label="decision 0015 · tokens" />
        <SrcRef file="docs/decisions/0125-kavach-for-ai-work.md" label="decision 0125 · Kavach for AI work" />
        <SrcRef file="docs/spec/13-ai-tooling.md" label="spec 13 · AI tooling" />
      </div>
    </>
  ),
};

export const conceptAgentsDoc: LessonDoc = {
  lead: "An AI agent in Kavach can do only what its code grants, within limits written beside the grant, and asks a person before what needs one. The runtime checks every call, and an audit log records it.",
  simple: <>Giving an AI agent a credit card is risky. Kavach lets you write, in the agent&apos;s code, exactly what it may buy, how much, and what needs your OK, and nothing can get around it.</>,
  minutes: 5,
  before: ["agents/agents"],
  body: (
    <>
      <H2>The problem</H2>
      <P>
        An agent acts in the world: it refunds, buys, deploys, sends mail. Its plan comes from a model, which can be
        wrong or be talked into things. A prompt saying &quot;never refund more than 500&quot; is a request, not a
        guarantee.
      </P>
      <H2>What other languages do</H2>
      <ul className={ul}>
        <li>Agent frameworks are libraries: the rules are code that runs if every path remembers to call it.</li>
        <li>Permissions are often all or nothing: the agent holds an API key that can do everything the key can.</li>
      </ul>
      <H2>What Kavach does</H2>
      <P>
        An agent is part of the language. <C>can</C> grants it capabilities, <C>limit</C> bounds each call and
        the totals over time, and <C>approve</C> makes a call wait for a person. The runtime checks them on every
        call, whatever the model said:
      </P>
      <Ex name="agents/uc-shopping" />
      <Ex name="agents/sub-approve" />
      <ul className={ul}>
        <li>A call outside <C>can</C> is a compile error, not a check that could be skipped.</li>
        <li>Every call, its verdict and its result can go to an audit log, and a dry run tries a plan without acting.</li>
        <li>A durable agent keeps its counts and waiting approvals across a restart.</li>
      </ul>
      <H2>What it costs</H2>
      <ul className={ul}>
        <li>The rules are written in code, so changing them is a code change, reviewed like any other.</li>
        <li>An action that waits for approval waits: the program must be built to let it.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0008-capabilities-in-signatures.md" label="decision 0008 · grants, limits, approvals" />
        <SrcRef file="docs/decisions/0072-approvals-that-wait-across-restarts.md" label="decision 0072 · approvals" />
        <SrcRef file="docs/spec/08-capabilities-and-agents/README.md" label="spec 8 · agents" />
      </div>
      <P>
        A whole agent, with its test: <Link href="/docs/guides/build-an-agent-with-limits-and-approvals/">Build an
        Agent with Limits and Approvals</Link>.
      </P>
    </>
  ),
};
