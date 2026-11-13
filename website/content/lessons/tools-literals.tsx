import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, Files, H2, P, Syntax } from "./parts";

// ---------- Tagged literals ----------

const LITERALS = "10-modules-and-packages/15-tagged-literals.md";
const misspelled = loadProject("literals/misspelled", ["src/main.kav"]);
const audited = loadProject("literals/audited", ["src/main.kav", "pg/src/lib.kav"]);

export const taggedLiteralsDoc: LessonDoc = {
  lead: "A package can give text a meaning the compiler checks: sql\"…\" is a Query, and a misspelled keyword in it is a compile error, not a failure in production.",
  simple: <>A tagged literal is like a form with a stamp on it: the stamp, <C>sql</C>, says what the text is, and the package that owns the stamp reads the text before the program ever runs, so a mistake is caught at your desk, not in front of a customer.</>,
  minutes: 5,
  before: ["tools/packages", "tools/modules/pub"],
  body: (
    <>
      <P>A package declares a tag with <C>literal</C>: its name, the type a literal of it makes, and the function that checks its text.</P>
      <Syntax code={'pub literal name: T check f language "x"'} />
      <P>
        Here the package <C>pg</C> declares <C>sql</C>. A <C>Query</C>&apos;s text isn&apos;t <C>pub</C>, so outside{" "}
        <C>pg</C> a <C>Query</C> comes only from a <C>sql&quot;…&quot;</C> literal:
      </P>
      <Files name="literals/sql" first={["pg/src/lib.kav", "src/main.kav"]} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>T</C> is a record whose first field is a <C>String</C>. The literal&apos;s value is a <C>T</C> holding its text; its other fields take their defaults.</li>
        <li>The tag is written right before the quote, with no space: <C>sql&quot;…&quot;</C> after <C>{"use pg.{sql}"}</C>, or <C>pg.sql&quot;…&quot;</C> through the package.</li>
        <li><C>language &quot;sql&quot;</C> is optional. It tells editors which language to colour inside the literal, as the window above does.</li>
      </ul>
      <H2>Checked when the program is compiled</H2>
      <P>
        The compiler runs the checker on each literal&apos;s text. Each problem it gives is reported at its place in
        the literal: an error, or a warning when its <C>warning</C> is true.
      </P>
      <CodeBlock code={misspelled.files[0].code} file="src/main.kav" />
      <Terminal session={misspelled.output ?? ""} />
      <P>The checker gives a <C>LiteralProblem</C> for each mistake. Every program has the type; it needs no <C>use</C>:</P>
      <CodeBlock code={specBlockWith(LITERALS, "10.15", "pub type LiteralProblem")} file="std.literals" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>at</C> counts characters of the text as written, from 0; <C>length</C> is how many the mark covers.</li>
        <li>In a literal over lines, <C>at</C> counts the text the checker gets, with the indentation the lines share removed. The problem is shown on its own line, as <C>updte</C> is above.</li>
        <li>
          The checker runs while the program is compiled, so it must be pure: <C>fn(text: String) -&gt; [LiteralProblem]</C>,
          with no <C>!</C> and no <C>uses</C> (<Link href="/docs/agents/capabilities/">capabilities</Link>). One that
          reads a file is refused.
        </li>
        <li>A checker that panics or runs longer than a second is a compile error that names the tag.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="tests/conformance/10-modules/tagged-literals-are-checked-when-compiled.kav" label="tested" />
        <SrcRef file="tests/conformance/10-modules/tagged-literal-problems-show-on-their-line.kav" label="over lines, tested" />
        <SrcRef file="tests/conformance/10-modules/a-tag-checker-uses-nothing.kav" label="pure checker, tested" />
      </div>
      <H2>The text is raw</H2>
      <P>
        Inside a tagged literal, <C>\</C> and <C>{"{"}</C> are characters like any other: no escapes and no
        interpolation, so values can&apos;t be pasted into it. Values go in separately, as <C>$1</C> does in SQL.
        Three quotes let a literal span lines, as other text does:
      </P>
      <Ex name="literals/raw" />
      <SrcRef file="tests/conformance/10-modules/tagged-literals-make-their-type.kav" label="tested" />
      <H2>Text known only when the program runs</H2>
      <P>
        Inside its package, code may still build a <C>T</C> from any text. A function that does so for text known only
        when the program runs, such as a migration read from a file, is marked <C>audited fn</C>.{" "}
        <C>kavach audit</C> lists each call of it, as it lists <C>reveal</C> and <C>unsafe</C>:
      </P>
      <Files name="literals/audited" first={["pg/src/lib.kav", "src/main.kav"]} terminal={false} />
      <Terminal session={audited.output ?? ""} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          Generated tests (<Link href="/docs/errors/testing/tests-from-types/">tests from types</Link>) don&apos;t make
          a <C>T</C> from made-up text. A function that takes one is tested through its <C>example</C> clauses and{" "}
          <C>test</C> blocks, which write literals.
        </li>
        <li>Joining two literals, <C>sql&quot;…&quot; + sql&quot;…&quot;</C>, is not part of this version.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${LITERALS}`} label="spec 10.15 · tagged literals" />
        <SrcRef file="tools/cli/tests/audit.rs" label="audit, tested" />
        <SrcRef file="docs/decisions/0100-tagged-literals.md" label="decision 0100" />
      </div>
    </>
  ),
};
