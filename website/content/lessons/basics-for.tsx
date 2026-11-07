import Link from "next/link";
import { C } from "@/components/inline-code";
import { RaceChart } from "@/components/race-chart";
import { SrcRef } from "@/components/src-ref";
import { VerifyBlock } from "@/components/verify-block";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P, Syntax } from "./parts";

// ---------- For Loop ----------

export const forDoc: LessonDoc = {
  lead: "A for loop runs a block of code once for each item: each item of a list, each number of a range, each entry of a map.",
  simple: <>A for loop is like a teacher calling the register: go down the list of names, one by one, and do the same thing for each. You don&apos;t keep count yourself; the list says when you are done.</>,
  minutes: 5,
  before: ["basics/variables", "basics/operators"],
  body: (
    <>
      <P>
        Use <C>for</C> when you know what you are going through. When you don&apos;t know how many rounds you
        need, use a <Link href="/docs/basics/while-loop/">while loop</Link> instead.
      </P>
      <H2>Syntax</H2>
      <Syntax code={"for item in things {\n    // runs once for each item\n}"} />
      <H2>Example</H2>
      <P>Each round, <C>name</C> holds the next item of the list:</P>
      <Ex name="for/list" />
      <Note>
        There is no counter to set up, test or increase, so there is no counter to get wrong.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Print the numbers 1 to 10, one per line.</>,
    answer: (
      <>
        <Ex name="for/one-to-ten" />
        <P><C>1..=10</C> includes the 10. With <C>1..10</C> the loop would stop at 9.</P>
      </>
    ),
  },
  deeper: (
    <>
      <P>
        <strong className="text-ink">No C-style loop.</strong> Kavach has no{" "}
        <C>for (i = 0; i &lt; n; i++)</C>. A range says the same thing with no condition to mistype and no step to
        forget, and when you need the position as well, <C>enumerate()</C> gives it.
      </P>
      <P>
        <strong className="text-ink">Big loops run on every core.</strong> The compiler asks of each counted loop:
        could the rounds run in any order and give the same answer? This one qualifies, because the compiler
        proves the sum taken <C>% 1_000_007</C> once is the same number:
      </P>
      <Ex name="for/modular" />
      <P>
        Such a loop is cut into chunks and spread over a pool of worker threads. If any check fails in a chunk,
        the chunks&apos; work is thrown away and the plain loop runs again on one core, so an overflow stops the
        program at exactly the round, message and line the simple loop would give.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/kir/src/par.rs" line={27} symbol="outline" />
        <SrcRef file="docs/decisions/0031-loops-on-all-cores.md" label="decision 0031" />
      </div>
      <P>The same loop with 500 million rounds, whole program:</P>
      <RaceChart suite="loops" id="loop" />
      <VerifyBlock suite="loops" id="loop" />
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>What does <C>for i in 1..4 {"{ print(i) }"}</C> print?</>,
      options: ["1  2  3", "1  2  3  4", "0  1  2  3"],
      answer: 0,
      explain: "The end of .. is left out; write 1..=4 to include it.",
    },
    {
      type: "order",
      prompt: <>Build a loop that prints each fruit: click the lines into order.</>,
      lines: ['for fruit in ["mango", "fig"] {', "    print(fruit)", "}"],
      explain: "Open the loop, do the work, close the block.",
    },
  ],
};

export const forListDoc: LessonDoc = {
  lead: "Loop over a list: the block runs once per item, in order, and the name you choose holds the item.",
  simple: <>Like taking things out of a shopping bag one at a time, first to last. The name you choose is your hand: each round, it holds the thing you are working with.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="for/list" />
      <P>
        Lists get their own page in <Link href="/docs/collections/lists/">Collections</Link>; here you only need
        the square brackets. A loop inside a loop goes through a list of lists:
      </P>
      <Ex name="for/nested" />
    </>
  ),
};

export const forRangeDoc: LessonDoc = {
  lead: "Loop over a range of numbers: .. leaves the end out, ..= puts it in.",
  simple: <>A range is a run of numbers, like the house numbers along a street. You choose whether the last number is included or left out.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="for/range" />
      <P>
        <C>0..3</C> is 0, 1, 2. <C>1..=3</C> is 1, 2, 3. Ranges that leave the end out join without overlap:{" "}
        <C>0..n</C> then <C>n..m</C> never counts <C>n</C> twice.
      </P>
      <H2>One past the end</H2>
      <P>
        A list&apos;s last index is <C>len() - 1</C>, so a loop to <C>..=items.len()</C> reads one item too many and
        panics on its last turn. The compiler warns before it runs:
      </P>
      <Ex name="for/one-past.warns" />
      <P>
        Write <C>0..items.len()</C>, or better, <C>for item in items</C>, which can&apos;t go past the end. A loop that
        changes the list, or reads it at another index, gets no warning.
      </P>
    </>
  ),
};

export const forMapDoc: LessonDoc = {
  lead: "Loop over a map: each round gives a key and its value together.",
  simple: <>A map is like a notebook of names and phone numbers, written down as you meet people. Looping over it gives you each name together with its number, one entry at a time, in the order they were written.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="for/map" />
      <P>
        <C>(name, age)</C> takes each entry apart into its two halves. The entries come in the order they were
        added, every time.
      </P>
    </>
  ),
};

export const forIndexDoc: LessonDoc = {
  lead: "When you need each item's position too, loop over list.enumerate().",
  simple: <>Like numbering the lines of a list as you read it: you get each item together with its position. Positions start at 0, not 1.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="for/index" />
      <P>Positions start at 0. You get the position and the item together, with no counter of your own.</P>
    </>
  ),
};

export const forTimesDoc: LessonDoc = {
  lead: "To repeat a block a number of times without using the number, name it _.",
  simple: <>Like stirring a pot a set number of times: you only care that it happens that many times, not which stir you are on. <C>_</C> says the number isn&apos;t needed.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="for/times" />
      <P>
        <C>_</C> says &quot;I don&apos;t need this value&quot;: it never becomes a variable, and a reader sees at
        once that the number itself doesn&apos;t matter.
      </P>
    </>
  ),
};
