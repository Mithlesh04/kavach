import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, Syntax } from "./parts";

const SPEC = "08-capabilities-and-agents/01-capabilities.md";

/** The built-in capabilities, read from the table of spec 8.1. */
function BuiltIns() {
  const table = specTables(SPEC, "8.1")[0];
  if (table.header.join("|") !== "Capability|Allows") throw new Error("spec 8.1: the capability table changed shape");
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Capability</th>
            <th className="py-2 font-bold">Allows</th>
          </tr>
        </thead>
        <tbody>
          {table.rows.map(([cap, allows]) => (
            <tr key={cap} className="border-b border-line">
              <td className="py-2 pr-4"><C>{cap.replace(/`/g, "")}</C></td>
              <td className="py-2 text-muted">{allows.replace(/\s*\(10\.7\)/, "")}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ---------- Capabilities ----------

export const capabilitiesDoc: LessonDoc = {
  lead: "A capability is permission to touch the outside world: files, the network, the terminal. A function says which ones it uses, and the compiler checks it.",
  simple: <>A capability is like a key to one door to the outside world: files, the network, the terminal. A function shows which keys it holds, and the compiler makes sure it never opens a door it has no key for.</>,
  minutes: 5,
  before: ["functions/functions", "errors/errors"],
  body: (
    <>
      <Syntax code={"pub fn name(…) -> Result ! Error uses Capability, Other {\n    …\n}"} />
      <Ex name="agents/cap-overview" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>total</C> has no <C>uses</C>: it can&apos;t touch anything outside the program. It only computes.</li>
        <li><C>save_total</C> says <C>uses Fs.write</C>: it may write files, and do nothing else outside.</li>
        <li><C>main</C> has every built-in capability, so it can call both.</li>
      </ul>
      <H2>The built-in capabilities</H2>
      <BuiltIns />
      <Note>
        Reading the first line of a function tells you everything it can reach. A library function that says no{" "}
        <C>uses</C> can&apos;t send your data anywhere: the compiler guarantees it, so you don&apos;t have to read its body.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Write <C>pub fn hello(name: String)</C> that prints a greeting. What must its first line say?</>,
    answer: <P>It prints, so it must say <C>uses Console</C>: <C>pub fn hello(name: String) uses Console</C>.</P>,
  },
  deeper: (
    <>
      <P>
        The check happens when the program is compiled, so there is no sandbox to escape while it runs: code that
        would reach further than it says never compiles. A caller can pass on only what it has itself, so a
        capability can&apos;t be picked up on the way down.
      </P>
      <div><SrcRef file={`docs/spec/${SPEC}`} label="spec 8.1 · capabilities" /></div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>A library you use has <C>pub fn format_price(p: Decimal) -&gt; String</C>, with no <C>uses</C>. What can it do?</>,
      options: [
        "Only compute its result: it can't read files, use the network or print",
        "Anything, as long as it returns a String",
        "Read files, but not use the network",
      ],
      answer: 0,
      explain: "A pub function must list what it touches. No uses means it touches nothing, and the compiler checks that.",
    },
  ],
};

export const usesDoc: LessonDoc = {
  lead: "A pub function must write its uses clause. A private one may leave it out: the compiler works it out.",
  simple: <><C>uses</C> is like the ingredients label on food: anything sold to others (<C>pub</C>) must carry one. A dish you only cook at home may skip it, and the compiler still knows what went in.</>,
  minutes: 3,
  body: (
    <>
      <P>A <C>pub</C> function that reads a file without saying so is refused, and the message names what to add:</P>
      <Ex name="agents/cap-pub.fails" />
      <P>
        A private function may leave <C>uses</C> out. The compiler works out what it touches, and that still counts
        for whoever calls it:
      </P>
      <Ex name="agents/cap-inferred" />
      <P>So a <C>pub</C> function that calls it must list it too:</P>
      <Ex name="agents/cap-inferred-caller.fails" />
    </>
  ),
};

export const pureDoc: LessonDoc = {
  lead: "A function without uses is pure: it can't affect the outside world at all.",
  simple: <>A pure function is like a calculator: it only works out an answer from what you give it, and can&apos;t touch anything else. The same input always gives the same answer, which makes it easy to test.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/cap-pure" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A pure function gives the same result for the same arguments, every time. That makes it easy to test.</li>
        <li><C>debug()</C> needs no capability: it writes to the error output, and release builds leave it out.</li>
        <li>Logging with <C>std.log</C> needs none either: the program&apos;s owner decides where log events go.</li>
      </ul>
    </>
  ),
};

export const fsNarrowDoc: LessonDoc = {
  lead: "Fs can be granted in part: Fs.read only reads, Fs.write only changes, and Fs(\"./data\") stays inside one folder.",
  simple: <>Like a key that opens only one room, or lets you look but not move anything. You give a function exactly the file access it needs: only reading, only writing, or only one folder.</>,
  minutes: 4,
  body: (
    <>
      <P><C>Fs.read</C> and <C>Fs.write</C> are checked by the compiler. A function that may only read can&apos;t write:</P>
      <Ex name="agents/cap-read-only.fails" />
      <P>
        A folder is checked while the program runs, since paths are values. Every file operation must name a path
        inside the folder, or it fails with <C>Denied</C>. <C>..</C> can&apos;t climb out:
      </P>
      <Ex name="agents/cap-folder" />
      <Note>
        The folder also covers everything the function calls, and the tasks it starts. A symbolic link that points
        outside the folder is refused too.
      </Note>
    </>
  ),
};

// ---------- Your Own Capabilities ----------

export const ownCapsDoc: LessonDoc = {
  lead: "Declare a capability for an outside system your code depends on, such as payments or email. Code calls it by name; who answers is decided elsewhere.",
  simple: <>Like a wall socket: your code plugs into &quot;payments&quot; without caring what is behind the wall. Tests plug in a fake and production plugs in the real one, and the code that calls it doesn&apos;t change.</>,
  minutes: 5,
  before: ["agents/capabilities", "types/interfaces"],
  body: (
    <>
      <Syntax code={"capability Name {\n    fn method(param: Type) -> Result ! ErrorWithDenied\n}"} />
      <Ex name="agents/own-implement" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>capability Payments</C> lists what the outside system can do, like an interface.</li>
        <li><C>TestBank</C> implements it. A real one would talk to the bank over the network.</li>
        <li>The agent <C>Shop</C> calls <C>Payments.charge</C> without knowing which implementation answers.</li>
      </ul>
      <Note>
        Tests hand in a fake implementation and production a real one, without changing the code that calls it.
        See <Link href="/docs/errors/testing/fakes/">Fakes</Link> for how a test hands one in.
      </Note>
    </>
  ),
};

export const declareCapDoc: LessonDoc = {
  lead: "A capability names methods. Every method can be refused, so its error type must have a Denied(reason: String) case.",
  simple: <>Declaring a capability is like writing the menu of what an outside service can do. Every item must allow for the answer &quot;no&quot;, because a limit, a person or a missing service can always refuse.</>,
  minutes: 3,
  body: (
    <>
      <P>
        Called where nothing implements it, a capability is refused: it never fails silently, and never does
        something unexpected:
      </P>
      <Ex name="agents/own-declare" />
      <P>
        A limit, a person who says no, or a missing implementation can refuse any call. So the compiler insists on
        the <C>Denied</C> case, and the refusal is visible to anyone reading the code:
      </P>
      <Ex name="agents/own-no-denied.fails" />
    </>
  ),
};

export const implementCapDoc: LessonDoc = {
  lead: "An implementation is an ordinary type with type Name: Capability, giving a body to each method.",
  simple: <>If the capability is the menu, the implementation is the kitchen that actually cooks each dish. It is an ordinary type, and it may need permissions of its own, such as reading files.</>,
  minutes: 3,
  body: (
    <>
      <P>
        An implementation may need capabilities of its own. This one reads files, and only inside{" "}
        <C>./inbox</C>, so the agent using it can&apos;t read anything else, whatever name it is given:
      </P>
      <Ex name="agents/uc-files" />
      <P>
        A real payments implementation would need <C>Net</C> to reach the bank. An agent is granted a capability
        whole (<C>can Payments</C>) or one method at a time (<C>can Inbox.read</C>).
      </P>
    </>
  ),
};

export const refusedCapDoc: LessonDoc = {
  lead: "A refused call fails with Denied(reason), an ordinary error: match on it like any other.",
  simple: <>A refused call is like a letter sent back with the reason stamped on it. You get an ordinary error that says why, and you handle it the same way as any other error.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="agents/own-refused" />
      <P>
        The reason says what refused it: a broken limit, a person who said no, or no implementation. Handle{" "}
        <C>Denied</C> apart from the system&apos;s own errors, such as a declined card or a bank that is down.
      </P>
      <P>
        The <C>_</C> arm is needed: <Link href="/docs/agents/running-an-agent/">a send can always fail</Link> with
        something else too, such as a panic in the handler.
      </P>
    </>
  ),
};
