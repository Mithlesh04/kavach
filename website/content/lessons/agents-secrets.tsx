import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { loadProject } from "@/lib/samples";
import { Ex, Note, P } from "./parts";

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
        <li>JSON: <C>json.stringify</C> of a record with a secret field is refused.</li>
      </ul>
      <P>Into a prompt, refused before the program runs:</P>
      <Ex name="agents/sub-secret.fails" />
      <Note>
        A card number in a prompt is a leak to a third party; a token in a log is a leak to everyone who reads logs.
        Kavach stops both when compiling, not after the leak.
      </Note>
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
      <Ex name="json/sub-secret.fails" />
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
        <C>reveal(order.card)</C> gives an ordinary <C>String</C>, here used to show the last four digits.{" "}
        <C>kavach audit</C> lists every <C>reveal</C> in a project, so none goes unnoticed:
      </P>
      <Terminal session={audited.output ?? ""} />
      <div><SrcRef file="docs/spec/08-capabilities-and-agents/07-secrets.md" label="spec 8.7 · secrets" /></div>
    </>
  ),
};
