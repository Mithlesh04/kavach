import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { OpTable, operatorGroups } from "./operator-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

const symbolsFix = loadProject("operators/symbols-fix", ["src/main.kav"]);

// ---------- Operators ----------

export const operatorsDoc: LessonDoc = {
  lead: "Operators compute with values: + adds, == compares, and combines two conditions, ?? picks a default.",
  simple: <>Operators are the small symbols and words that do the work between values, like the keys on a calculator: <C>+</C> adds, and <C>==</C> asks &quot;are these the same?&quot;. This page lists all of them, each with an example.</>,
  minutes: 6,
  before: ["basics/variables"],
  body: (
    <>
      <P>
        Every operator in Kavach, group by group: what it is called, what it does in plain words, and an example
        with its result. The results below were printed by the real compiler.
      </P>
      {operatorGroups.map((g) => (
        <OpTable key={g.title} {...g} />
      ))}
      <Note>
        Comparisons chain with any mix of <C>== != &lt; &lt;= &gt; &gt;=</C> and any number of values: each part
        compares its two neighbours, every value is worked out once, and the chain stops at the first part that is
        false. Brackets group as usual: <C>(a == b)</C> is worked out first, to <C>true</C> or <C>false</C>, and that
        answer is then compared as one value. Whole chains join with <C>and</C>, <C>or</C> and <C>not</C>, with no
        brackets needed, since comparisons go first: <C>a == b == c or d == e == d</C>. There is no <C>===</C> or{" "}
        <C>!==</C>: <C>==</C> and <C>!=</C> already compare contents, and types never mix.
      </Note>
      <P>
        Because a bracket gives a <C>Bool</C>, comparing it with a number is refused: <C>(a &lt; b) &lt; c</C> would
        ask whether <C>true</C> is less than <C>3</C>.
      </P>
      <Ex name="operators/group-chain.fails" />
      <Note>
        Logic is written in words, <C>and</C>, <C>or</C>, <C>not</C>, so a condition reads like a sentence. When
        operators mix, <Link href="/docs/basics/operators/precedence/">precedence</Link> decides which goes first;
        when in doubt, use brackets.
      </Note>
    </>
  ),
};

export const arithmeticDoc: LessonDoc = {
  lead: "+ - * / work as in school; % is the remainder and ** the power.",
  simple: <>These are the sums from school, written with the symbols on a keyboard: <C>*</C> for times and <C>/</C> for divide. One surprise: dividing two whole numbers gives a whole number, so <C>7 / 2</C> is <C>3</C>.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="operators/arithmetic.warns" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Two <C>Int</C>s divide to an <C>Int</C>: <C>7 / 2</C> is <C>3</C>. Use floats, <C>7.0 / 2.0</C>, for <C>3.5</C>.</li>
        <li>Written with two numbers that don&apos;t divide evenly, as <C>7 / 2</C> is here, it also gets a warning: it rarely means what it says. Write <C>7.div_floor(2)</C> when the whole part is what you want.</li>
        <li><C>2 ** 10</C> is 2 to the power 10.</li>
        <li>A whole-number result too big for its type stops the program; it never wraps to a wrong number.</li>
      </ul>
      <H2>Negative numbers: two ways to divide</H2>
      <Ex name="operators/floor.warns" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>/</C> and <C>%</C> round toward zero, as C and Java do: <C>-7 / 2</C> is <C>-3</C>, and <C>-7 % 3</C> is <C>-1</C>.</li>
        <li><C>div_floor</C> rounds down, and <C>mod</C> gives a remainder with the sign of the number you divide by, as Python&apos;s <C>{"//"}</C> and <C>%</C> do: <C>-4</C> and <C>2</C>.</li>
        <li>Use <C>mod</C> for anything that wraps around, such as days of the week: 3 days before day 1 is day <C>5</C>, never <C>-2</C>.</li>
      </ul>
    </>
  ),
};

export const assignmentDoc: LessonDoc = {
  lead: "= gives a variable a new value; += and the others change it in place.",
  simple: <>Assigning puts a new value into a variable, like refilling a jar you are allowed to open, a <C>let</C>. <C>score += 5</C> is the short way to say &quot;add 5 to what is already there&quot;.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="variables/change" />
      <P><C>score += 5</C> means <C>score = score + 5</C>. Only a <C>let</C> can be assigned to.</P>
    </>
  ),
};

export const comparisonDoc: LessonDoc = {
  lead: "== != < <= > >= compare two values and give a Bool. They chain, and == compares contents.",
  simple: <>A comparison asks a question about two values, like &quot;is this bag heavier than that one?&quot;, and the answer is always <C>true</C> or <C>false</C>. Two lists are equal when everything inside them is equal.</>,
  minutes: 4,
  body: (
    <>
      <Ex name="operators/comparison" />
      <P>There is one <C>==</C>, and it compares contents, so JavaScript&apos;s <C>===</C> isn&apos;t needed, and is refused with the fix:</P>
      <Ex name="habits/triple-eq.fails" />
      <H2>Chains</H2>
      <P>
        <C>18 &lt;= age &lt; 65</C> means <C>18 &lt;= age and age &lt; 65</C>, as in mathematics, and{" "}
        <C>age</C> is worked out only once. Any mix of comparisons chains, as long as you like:
      </P>
      <Ex name="operators/chain" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>a == b == 5</C> is <C>a == b and b == 5</C>: all three are equal.</li>
        <li><C>a == b != c</C> is <C>a == b and b != c</C>.</li>
        <li>
          Only neighbours are compared. <C>a != c != b</C> is true although <C>a</C> and <C>b</C> are equal: it
          checks <C>a != c</C> and <C>c != b</C>, never <C>a</C> against <C>b</C>.
        </li>
        <li>The chain stops at the first part that is false, and each value is worked out once.</li>
        <li>A chain can hold any number of values: <C>e &lt; c &lt; a == b &lt; d &lt; f == g</C> has seven.</li>
        <li>
          Brackets group: <C>(a == b) != (c == d)</C> works out each bracket first, then compares the two answers.
        </li>
      </ul>
      <H2>Contents, not places</H2>
      <P>
        <C>==</C> looks inside: two lists are equal when their items are, two maps when their entries are, in any
        order.
      </P>
      <H2>How comparisons behave</H2>
      <OpTable
        title="Case, text order and other cases worth knowing"
        sample="operators/t-behave"
        chain
        formHeader="Comparison"
        rows={[
          ["==", "Case counts", "Capital and small letters are different letters", '"Asha" == "asha"'],
          ["==", "Ignoring case", "Bring both sides to small letters first", '"Asha".lower() == "asha".lower()'],
          ["<", "Text order", "Text compares letter by letter, as in a dictionary", '"apple" < "banana"'],
          ["<", "Capitals first", "Letters compare by their character code, so every capital comes before every small letter", '"Zebra" < "apple"'],
          ["<", "Digits in text", "Text compares character by character, so \"10\" comes before \"9\"", '"10" < "9"'],
          ["<", "Numbers by value", "Numbers compare as numbers", "10 < 9"],
          ["<", "Characters", "One character against another, by its code", "'a' < 'b'"],
          ["==", "Same look, different text", "é as one character and as e plus an accent are different text: text is not normalized", '"\\u{e9}" == "e\\u{301}"'],
          ["==", "Missing values", "none equals none", "missing == none"],
          ["==", "Records", "Equal when all their fields are equal", 'User(name: "Asha") == User(name: "Asha")'],
        ]}
      />
      <P>Two kinds of number never compare silently: an <C>Int</C> against a <C>Float</C> is refused, with the fix:</P>
      <Ex name="operators/int-float.fails" />
    </>
  ),
  deeper: (
    <P>
      Two comparisons rarely mean what they say, and get a warning: <C>==</C> between floats worked out while the
      program runs (rounding makes them differ in the last bits; use <C>a.near(b)</C>), and{" "}
      <C>x % 2 == 1</C>, which misses negative odd numbers because <C>-3 % 2</C> is <C>-1</C> (write{" "}
      <C>x % 2 != 0</C>).
    </P>
  ),
};

export const logicalDoc: LessonDoc = {
  lead: "and is true when both sides are; or when either is; not turns true to false.",
  simple: <>These join yes-or-no questions the way we do when we talk. &quot;Sunny and warm&quot; needs both to be true; &quot;rain or snow&quot; is true if either one comes.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="operators/logical" />
      <P>The right side is worked out only when needed: in <C>a and b</C>, if <C>a</C> is false, <C>b</C> is skipped:</P>
      <Ex name="operators/short" />
      <H2>Words, not symbols</H2>
      <P>
        <C>!</C>, <C>&amp;&amp;</C> and <C>||</C>, from C, Java and JavaScript, are refused, and each message says
        which word to write. <C>kavach fix</C> writes them for you:
      </P>
      <CodeBlock code={symbolsFix.files[0].code} file="src/main.kav, before" />
      <Terminal session={symbolsFix.output ?? ""} />
    </>
  ),
};

export const precedenceDoc: LessonDoc = {
  lead: "* and / come before + and -, as in school. When in doubt, use parentheses.",
  simple: <>Like the rule from school that you multiply before you add, precedence decides which part of a sum goes first. Brackets are your way to say &quot;do this part first&quot;, and they make the order clear to anyone reading.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="operators/precedence" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>2 + 3 * 4</C> is 14: the multiplication goes first.</li>
        <li><C>-2 ** 2</C> is <C>-4</C>, as in mathematics: the power goes before the minus.</li>
        <li><C>2 ** 3 ** 2</C> is <C>2 ** 9</C>: powers go right to left.</li>
      </ul>
      <Note><C>kavach fmt</C> never removes parentheses that help a reader.</Note>
    </>
  ),
};

// ---------- Math ----------

export const mathDoc: LessonDoc = {
  lead: "std.math has square roots, powers, rounding, pi and more.",
  simple: <><C>std.math</C> is like a toolbox of ready-made calculations, such as square roots and rounding. You open it once at the top of your file with <C>use std.math</C>, then pick the tool you need.</>,
  minutes: 3,
  before: ["basics/operators"],
  body: (
    <>
      <Ex name="math/functions" />
      <P>Start the file with <C>use std.math</C>, then call each function as <C>math.name(…)</C>.</P>
    </>
  ),
};

export const mathFunctionsDoc: LessonDoc = {
  lead: "sqrt, pow, gcd, lcm, is_prime, factorial, mean, and the constants pi and e.",
  simple: <>These are the buttons of a scientific calculator, ready to use in code. Each one takes numbers and gives back an answer, such as the square root of a number.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="math/functions" />
      <ModuleTable file="05-math.md" number="14.5" first="Name" />
    </>
  ),
};

export const roundingDoc: LessonDoc = {
  lead: "floor rounds down, ceil rounds up, round rounds to the nearest, halves away from zero.",
  simple: <>Rounding turns a number with a fraction into a simpler one, like a shop rounding a price to the nearest rupee. You choose the direction: always down, always up, or to the nearest.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="math/rounding" />
      <P>They give a <C>Float</C> back. <C>Int(2.7)</C> turns a float into a whole number by cutting the fraction off.</P>
    </>
  ),
};

export const exactDoc: LessonDoc = {
  lead: "Numbers written in the code are worked out exactly, so 0.1 + 0.2 == 0.3 is true.",
  simple: <>When you type numbers straight into your code, Kavach works them out the way you would on paper, with no tiny errors. Numbers worked out while the program runs are different, and this page shows what to do with those.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="math/exact" />
      <P>
        The compiler works out <C>0.1 + 0.2</C> exactly and rounds once. Values computed while the program runs
        are rounded at each step, as on every computer: compare those with <C>near</C>, or use{" "}
        <Link href="/docs/basics/data-types/decimal/">Decimal</Link>.
      </P>
      <P>
        More number traps of other languages, and what Kavach does instead:{" "}
        <Link href="/docs/start/classic-mistakes-fixed/">Classic Mistakes, Fixed</Link>.
      </P>
    </>
  ),
};

// ---------- Booleans ----------

export const booleansDoc: LessonDoc = {
  lead: "A Bool is true or false. Comparisons make them, and and, or, not combine them.",
  simple: <>A <C>Bool</C> is a yes-or-no answer, like a light switch: on or off. Only a real yes-or-no counts as a condition; Kavach won&apos;t guess that a zero or empty text means &quot;no&quot;.</>,
  minutes: 3,
  before: ["basics/operators"],
  body: (
    <>
      <Ex name="datatypes/bool" />
      <Ex name="operators/logical" />
      <H2>Only a Bool is a condition</H2>
      <P>Some languages treat <C>0</C>, empty text or <C>none</C> as false. Kavach doesn&apos;t, and says what to write instead:</P>
      <Ex name="bool/truthy.fails" />
    </>
  ),
};
