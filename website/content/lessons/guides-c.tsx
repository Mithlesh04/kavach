import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Call a C Library Safely ----------
//
// One task from start to finish: guides/grades.project calls erf from C's
// math library, which std.math lacks, behind one safe Kavach function with
// contracts, and shows what kavach audit and kavach test say. Results are
// printed rounded, as C libraries differ in their last digits.

const grades = guideProject("grades", ["src/main.kav", "kavach.toml"]);
const main = "src/main.kav";

export const guideCDoc: LessonDoc = {
  lead: "One task from start to finish: use a function from a C library that Kavach doesn't have, behind one small, safe Kavach function with contracts, so the rest of the program never touches unsafe code, and kavach audit shows exactly where it is.",
  simple: <>Decades of useful code are written in C. Kavach can call it, but C doesn&apos;t check what Kavach checks. So the call goes in one marked place, wrapped in a function that checks its inputs and its answer, and the rest of the program uses that.</>,
  minutes: 12,
  before: ["other-languages/calling-c"],
  body: (
    <>
      <P>
        The task: given an exam&apos;s scores, with a mean of 60 and a standard deviation of 10, say what share of
        students each score beats. That takes the normal distribution, which needs <C>erf</C>, the error function.
        C&apos;s math library has it; <C>std.math</C> doesn&apos;t.
      </P>
      <H2>1. The package says it uses unsafe code</H2>
      <P>
        A call into C is <C>unsafe</C>: C doesn&apos;t check bounds, types or lifetimes. A package must say{" "}
        <C>unsafe = true</C> to use it at all, so anyone adding it as a dependency sees that it does:
      </P>
      <CodeBlock code={grades.file("kavach.toml")} file="kavach.toml" lang="toml" />
      <H2>2. Declare the C function</H2>
      <P>
        <C>extern c &quot;m&quot;</C> names the library, <C>libm</C>, and declares <C>erf</C> with its C types
        written as Kavach types: a C <C>double</C> is a <C>Float</C>.
      </P>
      <CodeBlock code={grades.piece(main, 'extern c "m"')} file={main} />
      <H2>3. One safe function around it</H2>
      <P>
        The only <C>unsafe</C> block in the program is inside <C>below</C>. Its <C>require</C> says what it takes,
        and its <C>ensure</C> says what it promises: a share between 0 and 1. Every caller gets those checks, and
        none of them writes <C>unsafe</C>:
      </P>
      <CodeBlock code={grades.piece(main, "fn below")} file={main} />
      <CodeBlock code={grades.piece(main, "fn main")} file={main} />
      <H2>4. Run it in native code</H2>
      <P>The interpreter can&apos;t call C, and says what to do instead:</P>
      <Terminal session={grades.said("run src/main.kav")} />
      <Terminal session={grades.said("run --native src/main.kav")} />
      <P>
        <C>kavach run --native</C> checks every contract on every call, so <C>below</C>&apos;s{" "}
        <C>require</C> and <C>ensure</C> are checked each time C answers. The shares are printed to one decimal,
        as C&apos;s math libraries may differ in their last digits from one system to another.
      </P>
      <H2>5. See where the unsafe code is</H2>
      <P>
        <C>kavach audit</C> lists every <C>unsafe</C> place, so a reviewer reads two lines instead of the whole
        program. Here they are the declaration and the one block:
      </P>
      <Terminal session={grades.said("audit")} />
      <H2>6. What the tests can check</H2>
      <Terminal session={grades.said("test --seed 7")} />
      <Note>
        <C>kavach test</C> runs in the interpreter, which can&apos;t call C, so it says it doesn&apos;t generate cases
        for <C>erf</C> or <C>below</C>, with the reason. For now <C>below</C> carries no <C>example</C>: one would run
        in the interpreter and fail. Spec 11.2 says such examples will be listed as not run instead, and checked once
        tests run in native code.
      </Note>
      <H2>The whole program</H2>
      <CodeBlock code={grades.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/10-modules-and-packages/07-interop.md" label="spec 10.7 · calling C" />
        <SrcRef file="docs/spec/06-memory.md" label="spec 6.4 · unsafe" />
        <SrcRef file="docs/spec/07-contracts.md" label="spec 7.4 · how contracts are checked" />
        <SrcRef file="website/content/samples/guides/grades.project/runs" label="the commands this page runs" />
      </div>
      <P>
        More: <Link href="/docs/other-languages/calling-c/">Calling C</Link> (text, records, callbacks, libraries
        loaded while running) and <Link href="/docs/other-languages/kavach-audit/">kavach audit</Link>.
      </P>
    </>
  ),
};
