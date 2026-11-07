import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, P, Syntax } from "./parts";

const story = loadProject("tools/deprecated", ["src/billing.kav", "src/main.kav"]);

// ---------- Deprecating a Function ----------

export const deprecateDoc: LessonDoc = {
  lead: "deprecated marks a function that is on its way out: every call warns, and kavach fix rewrites the calls to the new one.",
  simple: <>Like a sign on an old road that says &quot;use the new bridge&quot;: the old function still works, but everyone who uses it is told what to use instead, and <C>kavach fix</C> can send them the new way for you.</>,
  minutes: 4,
  before: ["tools/modules", "tools/the-kavach-command/fix"],
  body: (
    <>
      <Syntax code={'deprecated "use refund_order instead" fix refund_order(order)\npub fn refund(order: Order) -> Receipt ! RefundError uses Payments { … }'} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>deprecated</C> goes on its own line before the function: after its doc comment, before <C>pub</C>.</li>
        <li>The text in quotes says what to use instead. Every call shows it as a warning.</li>
        <li><C>fix</C>, which may be left out, is what each call becomes. The function&apos;s parameters in it stand for each call&apos;s arguments.</li>
      </ul>
      <H2>From warning to rewrite</H2>
      <P><C>half</C> is replaced by <C>half_of</C>. The old function stays, so no caller breaks on the day it changes:</P>
      <CodeBlock code={story.files[0].code} file="src/billing.kav" />
      <CodeBlock code={story.files[1].code} file="src/main.kav, before" />
      <Terminal session={story.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Both calls warn, each with the message from <C>deprecated</C>.</li>
        <li><C>billing.half(4)</C> becomes <C>billing.half_of(4)</C>: a call made through its module keeps the module.</li>
        <li>
          <C>half(6)</C> stays as it is, with its warning. It was brought in with <C>use billing.{"{half}"}</C>, so{" "}
          <C>half_of</C> isn&apos;t known there, and the rewrite wouldn&apos;t compile. <C>kavach fix</C> never makes
          an edit that adds an error. Add <C>half_of</C> to the <C>use</C>, and the next <C>kavach fix</C> rewrites it.
        </li>
        <li>The editor offers the same rewrite as a quick fix, one call at a time.</li>
      </ul>
      <H2>Arguments keep their meaning</H2>
      <P>
        Where the <C>fix</C> uses a parameter inside an operator, an argument that is more than a name, a literal, a
        field or a call goes in parentheses. With <C>fix half_of(n * 2)</C>, <C>half(a + b)</C> becomes{" "}
        <C>half_of((a + b) * 2)</C>, never <C>half_of(a + b * 2)</C>. A parameter the call leaves out stands for its
        default.
      </P>
      <H2>Only before a function</H2>
      <P>Today only functions can be deprecated. Anywhere else, <C>deprecated</C> is an error:</P>
      <Ex name="tools/deprecated-where.fails" />
      <P>
        A <C>fix</C> is code like any other, so a name in it that doesn&apos;t exist is the usual error, with a
        suggestion. Methods, types and fields can&apos;t be deprecated yet: that is an open question in spec 12.
      </P>
      <P>
        The other fixes <C>kavach fix</C> makes are on <Link href="/docs/tools/the-kavach-command/fix/">its own page</Link>.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/12-maintainability.md" label="spec 12.6 · deprecation" />
      </div>
    </>
  ),
};
