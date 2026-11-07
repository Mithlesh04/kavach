import { C } from "@/components/inline-code";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- User Input ----------

const typed = (lines: string) => (
  <p className="m-0 text-[14.5px] text-soft">
    Typed by the user: <C>{lines}</C>. What is typed isn&apos;t part of the program&apos;s output, so it doesn&apos;t
    show in the box.
  </p>
);

export const inputDoc: LessonDoc = {
  lead: "term.prompt(question) prints a question and gives back the line the user types.",
  simple: <>Input lets your program hold a conversation: it asks a question, waits, and uses the answer you type, like a shop assistant asking your name for an order. The user may also type nothing, and the program must be ready for that.</>,
  minutes: 4,
  before: ["basics/output", "basics/variables"],
  body: (
    <>
      <P>
        Asking the user lives in the <C>std.term</C> module, so a program that asks starts with{" "}
        <C>use std.term</C>.
      </P>
      <Ex name="input/text" />
      {typed("Asha")}
      <H2>Why the ?? &quot;friend&quot;</H2>
      <P>
        The user may type nothing at all and end the input (Ctrl+D, or a file that runs out). Then there is no
        answer, and <C>term.prompt</C> gives <C>none</C>. <C>?? &quot;friend&quot;</C> says what to use instead, so{" "}
        <C>name</C> is always text.
      </P>
      <Note>
        The program can&apos;t forget that case: using the answer without saying what to do when it is missing is
        a compile error.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Ask for the user&apos;s city and print &quot;Welcome to &lt;city&gt;&quot;, with &quot;your city&quot; when nothing is typed.</>,
    answer: <P><C>const city = term.prompt(&quot;Your city?&quot;) ?? &quot;your city&quot;</C>, then <C>print(&quot;Welcome to {"{city}"}&quot;)</C>.</P>,
  },
};

export const askTextDoc: LessonDoc = {
  lead: "term.prompt gives the line typed, as text, or none when the input has ended.",
  simple: <>It works like a form with one empty box: the question is printed, and you type your answer on the same line. Whatever you type comes back as text.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="input/text" />
      {typed("Asha")}
      <P>The question is printed with a space after it, and the answer is typed on the same line.</P>
    </>
  ),
};

export const askNumberDoc: LessonDoc = {
  lead: "term.prompt_int asks for a whole number, and asks again until it gets one.",
  simple: <>This asks for a number like a patient clerk at a counter: if you write &quot;thirty&quot;, it says that isn&apos;t a number and asks again. You get a real number back, ready to use in sums.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="input/number" />
      {typed("thirty, then 31")}
      <P>
        &quot;thirty&quot; isn&apos;t a number, so <C>prompt_int</C> says so and asks again. You never have to turn
        the text into a number yourself, or deal with an answer that isn&apos;t one. <C>term.prompt_float</C> does
        the same for numbers like <C>2.5</C>.
      </P>
    </>
  ),
};

export const askPasswordDoc: LessonDoc = {
  lead: "term.prompt_secret reads a password without showing it, and keeps it secret afterwards too.",
  simple: <>Like a cash machine that hides your PIN as you type, this reads a password without showing it. Even later, printing it shows only <C>&lt;secret&gt;</C>, so it can&apos;t slip into a log by mistake.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="input/password" />
      {typed("4321")}
      <P>
        The typing isn&apos;t shown on the screen. The answer is a <C>Secret</C>, and printing it shows{" "}
        <C>&lt;secret&gt;</C>, never the PIN: a password can&apos;t end up in a log by accident.
      </P>
      <P>
        <C>?? return</C> ends the program quietly if no PIN was typed.
      </P>
    </>
  ),
};
