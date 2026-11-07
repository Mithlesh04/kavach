import Link from "next/link";
import { C } from "@/components/inline-code";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P, Syntax } from "./parts";

// ---------- While Loop ----------

const neverEnds = loadProject("while/never-ends", ["src/main.kav"]);

export const whileDoc: LessonDoc = {
  lead: "A while loop repeats a block as long as its condition is true.",
  simple: <>Like filling a bucket from a tap: keep going while it isn&apos;t full. You don&apos;t know in advance how many rounds it takes, so you check before each one.</>,
  minutes: 4,
  before: ["basics/for-loop"],
  body: (
    <>
      <P>
        Use <C>while</C> when you don&apos;t know in advance how many rounds you need: until the fuel runs out,
        until the user types &quot;quit&quot;. When you are going through things you already have, a{" "}
        <Link href="/docs/basics/for-loop/">for loop</Link> is clearer.
      </P>
      <H2>Syntax</H2>
      <Syntax code={"while condition {\n    // runs again and again while condition is true\n}"} />
      <H2>Example</H2>
      <Ex name="while/while" />
      <Note>
        The condition is checked before every round. Something in the block must change it, or the loop never
        stops.
      </Note>
      <H2>A loop that never ends</H2>
      <P>When nothing in the loop changes its condition, and nothing leaves it, the compiler warns before the program runs:</P>
      <Terminal session={neverEnds.output ?? ""} />
      <P>Here <C>n</C> never changes. Add <C>n += 1</C> in the loop, or write <C>while true</C> if it really should run forever.</P>
    </>
  ),
  tryIt: {
    task: <>Start at 1 and keep doubling. Print the first number above 100.</>,
    answer: <Ex name="while/endless" />,
  },
  quiz: [
    {
      type: "type-output",
      prompt: <>What is the last line this prints?</>,
      code: 'let fuel = 3\nwhile fuel > 0 {\n    print("burn {fuel}")\n    fuel -= 1\n}\nprint("empty")',
      answer: "empty",
      explain: "When fuel reaches 0 the condition is false, the loop ends, and the line after it runs.",
    },
  ],
};

export const whilePlainDoc: LessonDoc = {
  lead: "while condition { … } checks the condition, runs the block, and checks again.",
  simple: <>Like driving with an eye on the fuel gauge: before each stretch you look, and you drive on only while there is fuel left.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="while/while" />
      <P>
        Round by round: <C>fuel</C> is 3, 2, 1, and the block runs each time. Then <C>fuel</C> is 0,{" "}
        <C>fuel &gt; 0</C> is false, and the program goes on after the loop.
      </P>
    </>
  ),
};

export const whileEndlessDoc: LessonDoc = {
  lead: "while true { … } runs until a break inside it stops the loop.",
  simple: <>Like searching a drawer for your keys: there is no end set in advance, and you stop the moment you find them. <C>break</C> is that moment.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="while/endless" />
      <P>
        Write <C>while true</C> when the place to stop is in the middle of the block, not at its start. The{" "}
        <C>break</C> leaves the loop; the next page shows it in detail.
      </P>
    </>
  ),
};

// ---------- Break/Continue ----------

export const breakDoc: LessonDoc = {
  lead: "break leaves a loop at once; continue skips the rest of this round and starts the next.",
  simple: <>Think of going through a pile of post. <C>break</C> is &quot;I found the letter I wanted, stop looking&quot;; <C>continue</C> is &quot;this one is an advert, skip it and take the next&quot;.</>,
  minutes: 4,
  before: ["basics/for-loop", "basics/while-loop"],
  body: (
    <>
      <H2>break</H2>
      <P>Stop looking once you have found what you need:</P>
      <Ex name="while/break" />
      <H2>continue</H2>
      <P>Skip the items you don&apos;t want, and carry on with the rest:</P>
      <Ex name="while/continue" />
      <Note>
        <C>break</C> and <C>continue</C> act on the innermost loop they are in. Both work in <C>for</C> and{" "}
        <C>while</C> loops.
      </Note>
      <H2>Leaving an outer loop: labels</H2>
      <P>
        To leave several loops at once, name the outer one: a label and <C>:</C> before <C>for</C> or{" "}
        <C>while</C>. Then <C>break outer</C> leaves it, from however deep inside:
      </P>
      <Ex name="while/labeled" />
      <P><C>continue</C> with a label goes on with that loop&apos;s next round, skipping the rest of the inner one:</P>
      <Ex name="while/label-continue" />
      <P>A plain <C>break</C> still leaves only the innermost loop. A label must name a loop around it, in the same function:</P>
      <Ex name="while/label-wrong.fails" />
      <Ex name="while/label-lambda.fails" />
    </>
  ),
  tryIt: {
    task: <>Add up the even numbers from 1 to 10, using <C>continue</C> to skip the odd ones.</>,
    answer: <Ex name="while/sum-even" />,
  },
  deeper: (
    <P>
      Labels work as in JavaScript, Java and Rust. A lambda or a <C>spawn</C> body is a function of its own, so a{" "}
      <C>break</C> inside one can&apos;t leave a loop outside it. When nested loops grow long, moving them into a
      function and using <C>return</C> is often clearer still: the function&apos;s name says what the loops are for.
    </P>
  ),
};
