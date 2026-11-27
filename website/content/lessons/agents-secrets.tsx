import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, H2, Note, P } from "./parts";

const audited = loadProject("agents/audit-cmd", ["src/main.kav"]);

// ---------- Secrets ----------

export const secretsDoc: LessonDoc = {
  lead: "Mark a field secret, and the compiler follows its value everywhere: it can't reach print, text, prompts, errors, logs or JSON unless you write reveal().",
  simple: <>Marking a value <C>secret</C> is like sealing it in an envelope marked &quot;private&quot;: it can&apos;t be printed, logged or sent to an AI by mistake. To open it, you must write <C>reveal()</C> on purpose.</>,
  minutes: 4,
  before: ["types/records"],
  body: (
    <>
      <Ex name="agents/secret-print.fails" />
      <P>A secret value must not flow into:</P>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>print</C>, <C>debug</C> or text with <C>{"{…}"}</C> in it,</li>
        <li>a prompt for <C>ask</C>, where it would leave for the model&apos;s provider,</li>
        <li>an error message, the audit log or any log,</li>
        <li>JSON: <C>json.stringify</C> writes a secret field as <C>&quot;&lt;secret&gt;&quot;</C>, its value never.</li>
      </ul>
      <P>Into a prompt, refused before the program runs:</P>
      <Ex name="agents/sub-secret.fails" />
      <Note>
        A card number in a prompt is a leak to a third party; a token in a log is a leak to everyone who reads logs.
        Kavach stops both when compiling, not after the leak.
      </Note>
      <P>
        A value read from the environment with <C>env(name)</C> is a secret too, since keys and passwords usually come
        that way: see <Link href="/docs/tools/environment-variables/">Environment Variables</Link>.
      </P>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>You really must show part of a secret once. What do you write?</>,
      options: ["reveal(value): easy to spot in review, and listed by kavach audit", "Nothing special: print it", "Turn secrets off in kavach.toml"],
      answer: 0,
      explain: "Revealing is allowed, but never silent.",
    },
  ],
};

export const secretFieldsDoc: LessonDoc = {
  lead: "secret before a field marks it. Logs show such a field as <secret>.",
  simple: <>Like a form where one box is blacked out on every copy: the log shows all the other fields, but the secret one appears only as <C>&lt;secret&gt;</C>.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/sub-log" />
      <P>The rest of the record is logged as usual, so the entry is still useful. Only the card is hidden.</P>
      <P>JSON hides it the same way:</P>
      <Ex name="json/sub-secret" />
    </>
  ),
};

export const secretParamsDoc: LessonDoc = {
  lead: "A secret can go to a parameter that is secret itself, and nowhere else.",
  simple: <>A secret can only be handed to someone who promises to keep it secret too. A parameter marked <C>secret</C> is that promise; an ordinary parameter can&apos;t take it, and a copy of a secret is a secret as well.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/secret-param" />
      <P>
        Inside <C>charge</C>, <C>card</C> is a secret like the field it came from. So is a variable set from a
        secret: here <C>c</C> is, and an ordinary parameter can&apos;t take it:
      </P>
      <Ex name="agents/secret-remember.fails" />
      <H2>Anything made from a secret</H2>
      <P>
        A value made from a secret is a secret too, whatever it computes. A method&apos;s result is, even the
        card&apos;s length:
      </P>
      <Ex name="agents/secret-made.fails" />
      <P>
        So is a list, tuple, dict or record holding one, text built from one, and an <C>if</C> or <C>match</C> whose
        branch gives one. Storing a secret in a part of a
        variable, as <C>cards[0] = …</C>, makes the whole variable secret:
      </P>
      <Ex name="agents/secret-list.fails" />
      <P>
        A variable given a secret is secret everywhere in its function, even above that line, as a loop may come
        back to it:
      </P>
      <Ex name="agents/secret-above.fails" />
      <P>The names a <C>for</C>, <C>match</C>, <C>if let</C> or destructuring takes from a secret are secret too:</P>
      <Ex name="agents/secret-loop.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A built-in method&apos;s parameters aren&apos;t <C>secret</C>, so <C>cards.push(card)</C> is refused.</li>
        <li>A field that isn&apos;t <C>secret</C> can&apos;t take one, when the record is made or later.</li>
        <li>A lambda given to a method of a secret may use its secrets: in <C>{"cards.map(c => c.upper())"}</C> the result is secret anyway.</li>
      </ul>
      <P>
        Only what is computed counts. The condition of an <C>if</C>, or the value a <C>match</C> looks at,
        doesn&apos;t make the result secret, and <C>reveal</C> makes a value public where you choose:
      </P>
      <Ex name="agents/secret-condition" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/07-secrets.md" label="spec 8.7 · made from a secret" />
        <SrcRef file="tests/conformance/08-capabilities/secret-method-results-are-secret.kav" label="method results, tested" />
        <SrcRef file="tests/conformance/08-capabilities/storing-a-secret-in-a-list-makes-it-secret.kav" label="parts, tested" />
        <SrcRef file="tests/conformance/08-capabilities/variables-are-secret-above-the-assignment.kav" label="above the line, tested" />
        <SrcRef file="tests/conformance/08-capabilities/loops-over-a-secret-take-secrets.kav" label="loops, tested" />
        <SrcRef file="tests/conformance/08-capabilities/branches-giving-a-secret-are-secret.kav" label="branches, tested" />
      </div>
    </>
  ),
};

export const secretResultsDoc: LessonDoc = {
  lead: "A function that returns a secret says so in its result: -> secret T. Its caller gets a secret, like the field it came from.",
  simple: <>A function that hands back a secret must say so on its label, so the person who gets it knows to keep it secret too. A helper can&apos;t quietly turn a secret into ordinary text.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/secret-result" />
      <P>Returning a secret from a function whose result isn&apos;t <C>secret</C> is refused:</P>
      <Ex name="agents/secret-result-plain.fails" />
      <P>And what the caller gets stays a secret:</P>
      <Ex name="agents/secret-result-print.fails" />
      <P>
        A lambda can&apos;t return a secret: a function type has no way to say so. Write a named function with{" "}
        <C>-&gt; secret T</C> instead:
      </P>
      <Ex name="agents/secret-result-lambda.fails" />
      <P>Interface methods and the methods that implement them agree on it.</P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/07-secrets.md" label="spec 8.7 · returning a secret" />
        <SrcRef file="tests/conformance/08-capabilities/returning-a-secret-says-so.kav" label="tested" />
        <SrcRef file="tests/conformance/08-capabilities/secret-results-stay-secret.kav" label="stays secret, tested" />
        <SrcRef file="tests/conformance/08-capabilities/lambdas-return-no-secrets.kav" label="lambdas, tested" />
        <SrcRef file="docs/decisions/0094-secret-results.md" label="decision 0094" />
      </div>
    </>
  ),
};

export const revealDoc: LessonDoc = {
  lead: "reveal(value) lets a secret through on purpose, where review can see it.",
  simple: <><C>reveal</C> is like signing your name when you open a sealed envelope: it is allowed, but it leaves a mark. <C>kavach audit</C> lists every place it happens.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/secret-param" />
      <P>
        <C>reveal(order.card)</C> gives an ordinary <C>String</C>, here used to show the last four digits. Anything
        made from a secret is secret too, a card&apos;s length included, so <C>charge</C> writes{" "}
        <C>reveal(card.len())</C> to put it in its text. <C>kavach audit</C> lists every <C>reveal</C> in a project,
        so none goes unnoticed:
      </P>
      <Terminal session={audited.output ?? ""} />
      <div><SrcRef file="docs/spec/08-capabilities-and-agents/07-secrets.md" label="spec 8.7 · secrets" /></div>
    </>
  ),
};
