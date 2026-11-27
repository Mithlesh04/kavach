import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, Files, H2, Note, P, Syntax } from "./parts";
import { Cell } from "./spec-table";

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
              <td className="py-2 text-muted"><Cell text={allows.replace(/\s*\(\d+\.\d+\)/, "")} /></td>
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
        <li><C>total</C> has no <C>uses</C>: it can&apos;t read files, use the network or touch anything else outside the program. Like every function, it could print: <Link href="/docs/agents/capabilities/default-capabilities/"><C>Console</C> is a default</Link>.</li>
        <li><C>save_total</C> says <C>uses Fs.write</C>: it may write files, and do nothing else outside.</li>
        <li><C>main</C> has every built-in capability, so it can call both.</li>
      </ul>
      <H2>The built-in capabilities</H2>
      <BuiltIns />
      <Note>
        Reading the first line of a function tells you everything it can reach. A library function that says no{" "}
        <C>uses</C> can at most print: it can&apos;t read your files or send your data anywhere. The compiler
        guarantees it, so you don&apos;t have to read its body.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Write <C>pub fn save(name: String)</C> that writes the name to a file. What must its first line say?</>,
    answer: <P>It writes a file, so it must say <C>uses Fs.write</C>: <C>pub fn save(name: String) ! FsError uses Fs.write</C>. Printing would need nothing written, as <C>Console</C> is a default capability.</P>,
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
        "Compute its result, and print: it can't read files or use the network",
        "Anything, as long as it returns a String",
        "Read files, but not use the network",
      ],
      answer: 0,
      explain: "A pub function must list what it touches beyond the package's defaults, which are Console unless kavach.toml says otherwise. No uses means it touches nothing else, and the compiler checks that.",
    },
  ],
};

export const usesDoc: LessonDoc = {
  lead: "A pub function must write its uses clause for what it uses beyond the package's defaults. A private one may leave it out: the compiler works it out.",
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
      <H2>Field defaults count where a value is made</H2>
      <P>
        A field&apos;s default value is worked out when a value is made without that field. So the code that makes
        it needs what the default uses. Here <C>Report()</C> runs <C>disk_size()</C>, which reads a file:
      </P>
      <Ex name="agents/cap-field-default.fails" />
    </>
  ),
};

export const defaultCapsDoc: LessonDoc = {
  lead: "Every function in a package may use its default capabilities without writing uses. The default is Console, so print needs no uses; kavach.toml can change it.",
  simple: <>Some permissions are so common that asking for each one would only add noise. Printing is one: every function may print without saying so. A project that wants every effect written down can turn that off.</>,
  minutes: 4,
  before: ["agents/capabilities/uses"],
  body: (
    <>
      <P>
        <C>greet</C> is <C>pub</C> and prints, but writes no <C>uses</C>: <C>Console</C> is the default, so every
        function may use it:
      </P>
      <Ex name="agents/cap-default" />
      <P>Writing <C>uses Console</C> anyway is allowed, and changes nothing.</P>
      <H2>Choosing the defaults</H2>
      <P>
        <C>[capabilities] default</C> in <C>kavach.toml</C> lists them. Left out, it is{" "}
        <C>[&quot;Console&quot;]</C>. Several may be listed. <C>default = []</C> takes them all away, so every
        effect is written in a <C>uses</C>, printing too:
      </P>
      <Files name="agents/cap-default-none.fails" first={["kavach.toml", "src/main.kav"]} outcome="refused" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A default capability is part of every signature other packages see. A package without it must write <C>uses</C> to call code that has it.</li>
        <li>An agent gets no defaults: it may use only what its <Link href="/docs/agents/agents/can/"><C>can</C></Link> lists, <C>Console</C> too.</li>
      </ul>
      <H2>What can&apos;t be a default</H2>
      <P>
        <C>System.critical</C>, <C>Approvals</C> and <C>Foreign</C> are always written by name, in a{" "}
        <C>uses</C>, so a reader always sees them:
      </P>
      <Files name="agents/cap-default-foreign.fails" first={["kavach.toml", "src/main.kav"]} outcome="refused" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/decisions/0148-default-capabilities.md" label="decision 0148 · default capabilities" />
        <SrcRef file={`docs/spec/${SPEC}`} label="spec 8.1 · rule 7" />
      </div>
    </>
  ),
};

export const pureDoc: LessonDoc = {
  lead: "A function that uses no capability is pure: it can't affect the outside world at all. With default = [] in kavach.toml, every function without uses is.",
  simple: <>A pure function is like a calculator: it only works out an answer from what you give it, and can&apos;t touch anything else. The same input always gives the same answer, which makes it easy to test.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="agents/cap-pure" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A pure function gives the same result for the same arguments, every time. That makes it easy to test.</li>
        <li>Without <C>uses</C>, a function may still print, as <Link href="/docs/agents/capabilities/default-capabilities/"><C>Console</C> is a default</Link>. A project that sets <C>default = []</C> makes every function without <C>uses</C> pure.</li>
        <li><C>debug()</C> needs no capability: it writes to the error output, and release builds leave it out.</li>
        <li>Logging with <C>std.log</C> needs none either: the program&apos;s owner decides where log events go.</li>
      </ul>
      <H2>Functions given as values</H2>
      <P>
        A function value carries the rights of the code that made it. Naming a function as a value, or writing a
        lambda, needs what that function uses, as calling it would. Calling a function value needs nothing more:
      </P>
      <Ex name="agents/cap-fn-values" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>twice</C> stays pure: it only calls what it is given.</li>
        <li><C>shout</C> writes the lambda that prints, so <C>shout</C> needs <C>Console</C>.</li>
        <li>So a library can call the functions it is given, as a web framework calls its handlers, without being granted what they use.</li>
      </ul>
      <P>A function type can set a limit with <C>uses</C>: a function value may then use only what the type lists.</P>
      <Ex name="agents/cap-fn-type.fails" />
      <P>
        <C>uses ()</C> lists nothing: such a function value may use no capability at all, not even the
        package&apos;s defaults. That is what a <Link href="/docs/errors/contracts/require/">contract</Link> needs
        to call one:
      </P>
      <Ex name="agents/cap-fn-type-nothing.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/08-capabilities-and-agents/01-capabilities.md" label="spec 8.1 · capabilities" />
        <SrcRef file="tests/conformance/08-capabilities/function-values-carry-their-makers-rights.kav" label="tested" />
        <SrcRef file="tests/conformance/08-capabilities/function-types-limit-what-functions-use.kav" label="limit tested" />
      </div>
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
