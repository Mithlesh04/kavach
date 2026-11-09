import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P, Syntax } from "./parts";

// ---------- If...Else ----------

export const ifDoc: LessonDoc = {
  lead: "if runs a block only when its condition is true; else gives the other way.",
  simple: <>An if is a fork in the road: if it is raining, take an umbrella; otherwise, wear sunglasses. The program asks a yes-or-no question and goes one way or the other.</>,
  minutes: 4,
  before: ["basics/operators", "basics/booleans"],
  body: (
    <>
      <H2>Syntax</H2>
      <Syntax code={"if condition {\n    // when condition is true\n} else {\n    // otherwise\n}"} />
      <H2>Example</H2>
      <Ex name="if/else-if" />
      <P>The conditions are tried top to bottom, and only the first true one runs its block.</P>
      <Note>
        The condition must be a <C>Bool</C>. <C>if stock</C> is an error; write what you mean, such as{" "}
        <C>if stock &gt; 0</C>.
      </Note>
      <H2>= is not ==</H2>
      <P>
        <C>=</C> sets a variable, and only as a statement of its own. In a condition it is an error that says what to
        write instead. The same holds for <C>while</C>:
      </P>
      <Ex name="if/equals.fails" />
      <H2>A branch that can never run</H2>
      <P>Conditions are tried in order, so a wider one first hides a narrower one after it. The compiler warns:</P>
      <Ex name="if/never-true.warns" />
      <P>
        In the <Link href="/docs/tools/editor-setup/">editor</Link>, the quick fix &quot;Remove this branch&quot; takes
        out the <C>else if</C> and its block. An <C>else</C> after it stays, joined to the branch before.
      </P>
    </>
  ),
  tryIt: {
    task: <>Given a number <C>n</C>, print whether it is even or odd.</>,
    answer: <Ex name="if/even-odd" />,
  },
  quiz: [
    {
      type: "type-output",
      prompt: <>What does this print?</>,
      code: 'const stock = 0\nif stock == 0 {\n    print("Sold out")\n} else if stock < 5 {\n    print("Few left")\n} else {\n    print("In stock")\n}',
      answer: "Sold out",
      explain: "The first true condition wins; the others aren't even looked at.",
    },
  ],
};

export const ifOnlyDoc: LessonDoc = {
  lead: "if on its own: the block runs when the condition is true, and is skipped when it isn't.",
  simple: <>Like a note that says &quot;if the milk is finished, buy more&quot;: when it is true, you do it; when it isn&apos;t, you just carry on with the rest of your day.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="if/if" />
      <P>Change <C>34</C> to <C>20</C> and only <C>Done</C> is printed.</P>
    </>
  ),
};

export const elseIfDoc: LessonDoc = {
  lead: "else if tries another condition; the final else catches everything left.",
  simple: <>Like sorting post: if it is for this town, one tray; else if it is for this country, another tray; else, the tray for abroad. The first rule that fits wins, and the last tray takes the rest.</>,
  minutes: 2,
  body: <Ex name="if/else-if" />,
};

export const ifValueDoc: LessonDoc = {
  lead: "An if is a value: it picks between two values in one line.",
  simple: <>Sometimes you don&apos;t want to do something, only to pick a word or a number, like choosing a &quot;child&quot; or &quot;adult&quot; ticket by age. An if can give that choice back as a value, in one line.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="if/value" />
      <P>
        Each block&apos;s last line is its value. This replaces the <C>? :</C> operator of other languages, with
        words a beginner can read. Both branches must give the same type.
      </P>
    </>
  ),
};

// ---------- If Let ----------

export const ifLetDoc: LessonDoc = {
  lead: "if let runs a block only when an optional value is there, and gives you the value inside.",
  simple: <>Like opening a parcel box that may be empty: if there is something inside, take it out and use it; if not, skip that part.</>,
  minutes: 3,
  before: ["basics/if-else"],
  body: (
    <>
      <Ex name="if/iflet" />
      <P>
        <C>ages[&quot;asha&quot;]</C> may find nothing, so its type is <C>Int?</C>: an <C>Int</C> or <C>none</C>.{" "}
        <C>if let age = …</C> runs the block only when there is a value, and inside it <C>age</C> is a plain{" "}
        <C>Int</C>.
      </P>
      <Note>
        Optional values have their own page: <Link href="/docs/types/optionals/">Optionals</Link>.
      </Note>
    </>
  ),
};

// ---------- Match ----------

export const matchDoc: LessonDoc = {
  lead: "match compares a value against patterns, top to bottom, and runs the first that fits.",
  simple: <>Like a coin sorting machine: each coin rolls past the slots in order and drops into the first one it fits. A <C>match</C> does the same with a value, and makes sure every possible value has a slot.</>,
  minutes: 4,
  before: ["basics/if-else"],
  body: (
    <>
      <H2>Syntax</H2>
      <Syntax code={"match value {\n    pattern => result\n    pattern => result\n    _ => result      // anything else\n}"} />
      <H2>Example</H2>
      <Ex name="match/values" />
      <P>
        Like <C>if</C>, a <C>match</C> is a value. <C>_</C> matches anything, so it goes last.
      </P>
      <H2>Every case, or an error</H2>
      <P>A <C>match</C> must cover every possible value. Forget one, and the compiler names it:</P>
      <Ex name="match/missing.fails" />
      <H2>An arm that can never match</H2>
      <P>Arms are tried in order too. An arm after one that matches everything is dead code, and the compiler says so:</P>
      <Ex name="match/never-matches.warns" />
      <P>In the <Link href="/docs/tools/editor-setup/">editor</Link>, the quick fix &quot;Remove this arm&quot; takes out the arm&apos;s whole lines, comma included.</P>
      <H2>An arm that leaves</H2>
      <P>
        An arm may leave instead of giving a value: <C>return</C>, <C>fail</C>, <C>break</C> or <C>continue</C>. The{" "}
        <C>match</C> then has the value of the arm that was taken, and the code after it runs only when one that gives a
        value was:
      </P>
      <Ex name="match/leave" />
      <P>In a loop, <C>break</C> stops the loop and <C>continue</C> goes on to the next round:</P>
      <Ex name="match/leave-loop" />
      <P>200 stops the loop, so 7 is never added, and -3 is skipped: 5 + 10 is 15.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/04-control-flow.md" label="spec 4.3 · match" />
        <SrcRef file="tests/conformance/04-control-flow/a-match-arm-may-leave.kav" label="its conformance test" />
      </div>
    </>
  ),
  tryIt: {
    task: <>Match a number from 1 to 3 to the words &quot;one&quot;, &quot;two&quot;, &quot;three&quot;, and &quot;many&quot; for anything else.</>,
    answer: <P>Four arms: <C>1 =&gt; &quot;one&quot;</C>, <C>2 =&gt; &quot;two&quot;</C>, <C>3 =&gt; &quot;three&quot;</C>, and <C>_ =&gt; &quot;many&quot;</C> last.</P>,
  },
};

export const matchValuesDoc: LessonDoc = {
  lead: "Match exact values: numbers, text, true and false.",
  simple: <>The simplest pattern is one exact value, like ordering from a menu by number: 1 means soup, 2 means rice. The value must be exactly that number, that text, or true or false.</>,
  minutes: 2,
  body: <Ex name="match/values" />,
};

export const patternsDoc: LessonDoc = {
  lead: "Patterns can be ranges, tuples, union variants, none, or _ for anything.",
  simple: <>A pattern can stand for a whole group of values, like a teacher&apos;s rule &quot;90 to 100 marks is an A&quot;. Other patterns look at the parts of a pair, or say &quot;anything at all&quot; with <C>_</C>.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="match/patterns" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>90..=100</C> matches any number in the range.</li>
        <li><C>(0, y)</C> matches a pair whose first item is 0, and names the second <C>y</C>.</li>
        <li>A union variant matches its own case: <Link href="/docs/types/records/">records and unions</Link> show it.</li>
      </ul>
    </>
  ),
};

export const guardsDoc: LessonDoc = {
  lead: "A guard, pattern if condition, adds a check to a pattern.",
  simple: <>A guard is an extra question on top of a pattern, like a doorman who first asks &quot;is this a ticket?&quot; and then &quot;is it for today?&quot;. Only when both answers are yes does that arm run.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="match/guards" />
      <P>
        The square case is a <C>Rect</C> whose sides are equal. The plain <C>Rect</C> arm after it catches the
        rest, so every shape is still covered.
      </P>
    </>
  ),
};
