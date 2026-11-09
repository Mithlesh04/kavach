// What Kavach does for you without being asked: one list, written once.
// Each entry shows in a "What Kavach does for you" box on the page that
// teaches it (`where`, a course key), and all of them together on
// start/what-kavach-does-for-you. Every entry links its proof: the spec
// rule, the decision, or the source file. lib/site.ts fails the build
// when a proof's file no longer exists.

import type { ReactNode } from "react";
import { C } from "@/components/inline-code";

export type SmartGroup = "Fewer mistakes" | "Numbers and types" | "Clear messages" | "Fast without effort" | "Safe by default";

export type Proof = { file: string; label: string };

export type SmartThing = {
  group: SmartGroup;
  title: string;
  what: ReactNode;
  /** The course page or sub-page that teaches it. */
  where: string;
  /** A file in the repository, from its root, and the link text. */
  proof: Proof;
};

const spec = (file: string, label: string): Proof => ({ file: `docs/spec/${file}`, label });
const decision = (file: string, label: string): Proof => ({ file: `docs/decisions/${file}`, label });

export const smart: SmartThing[] = [
  // ---------- Fewer mistakes ----------
  {
    group: "Fewer mistakes",
    title: "Comparisons chain, as in mathematics",
    what: <><C>0 &lt;= i &lt; items.len()</C> means both tests, and the middle is worked out once. Any mix chains: <C>a == b != c</C> is <C>a == b and b != c</C>. No <C>and</C> to forget.</>,
    where: "basics/operators/comparison",
    proof: decision("0040-chained-comparisons.md", "decision 0040"),
  },
  {
    group: "Fewer mistakes",
    title: "== compares contents",
    what: <>Two lists are equal when their items are; two maps when their entries are, in any order. A copy that was never changed is equal in one step, without looking inside.</>,
    where: "basics/operators/comparison",
    proof: decision("0039-equality-is-deep-and-fast.md", "decision 0039"),
  },
  {
    group: "Fewer mistakes",
    title: "No truthiness",
    what: <>A condition must be a <C>Bool</C>. <C>if count</C> is an error, so <C>0</C>, <C>&quot;&quot;</C> and <C>none</C> can never be taken for false by accident.</>,
    where: "basics/if-else",
    proof: spec("04-control-flow.md", "spec 4.1"),
  },
  {
    group: "Fewer mistakes",
    title: "match covers every case",
    what: <>Add a new variant to a union and the compiler lists every <C>match</C> that must now handle it. Nothing is forgotten silently.</>,
    where: "basics/match",
    proof: spec("04-control-flow.md", "spec 4.3"),
  },
  {
    group: "Fewer mistakes",
    title: "Maps and sets keep their order",
    what: <>Looping over a map visits entries in the order they were added, so a program prints the same thing every run, and tests stay repeatable.</>,
    where: "basics/for-loop/over-a-map",
    proof: spec("02-types/02-collections.md", "spec 2.2"),
  },
  {
    group: "Fewer mistakes",
    title: "Ranges say which end they mean",
    what: <><C>0..10</C> stops before 10; <C>0..=10</C> includes it. No <C>n - 1</C> or <C>n + 1</C> to get wrong.</>,
    where: "basics/for-loop/over-a-range",
    proof: spec("01-basics.md", "spec 1.8"),
  },
  {
    group: "Fewer mistakes",
    title: "const unless it must change",
    what: <><C>const</C> values never change, and changing one is a compile error that says to use <C>let</C>. You see at a glance what can move.</>,
    where: "basics/variables/const",
    proof: decision("0034-const-and-let.md", "decision 0034"),
  },
  {
    group: "Fewer mistakes",
    title: "Taking values apart is typed",
    what: <><C>const [first, second] = scores</C> works on any list: a missing item becomes <C>none</C>, and its type says so.</>,
    where: "basics/variables/destructuring",
    proof: decision("0037-destructuring-as-javascript-does.md", "decision 0037"),
  },
  {
    group: "Fewer mistakes",
    title: "Comments nest",
    what: <><C>{"/* … */"}</C> may hold other block comments, so code that has comments can be put aside whole.</>,
    where: "basics/comments",
    proof: spec("01-basics.md", "spec 1.2"),
  },
  // ---------- Numbers and types ----------
  {
    group: "Numbers and types",
    title: "Overflow is caught, never wrapped",
    what: <>An integer that goes past its type&apos;s limit stops the program with a clear message, instead of quietly turning into a wrong number. When the compiler knows the values, even through a loop over known numbers, it is a compile error.</>,
    where: "basics/operators/arithmetic",
    proof: spec("01-basics.md", "spec 1.8"),
  },
  {
    group: "Numbers and types",
    title: "Constants are exact",
    what: <><C>0.1 + 0.2 == 0.3</C> is <C>true</C> when both are constants: the compiler works constants out exactly, then rounds once.</>,
    where: "basics/math/exact-numbers",
    proof: decision("0043-constants-are-exact.md", "decision 0043"),
  },
  {
    group: "Numbers and types",
    title: "A number takes its type from where it goes",
    what: <><C>const price: Decimal = 499.99</C> is an exact decimal, not a rounded float. Without a type, whole numbers are <C>Int</C>.</>,
    where: "basics/data-types/decimal",
    proof: spec("01-basics.md", "spec 1.7"),
  },
  {
    group: "Numbers and types",
    title: "Decimal never runs out of digits",
    what: <>A <C>Decimal</C> keeps as many digits as it needs, so money adds up to the paisa however large it gets.</>,
    where: "basics/data-types/decimal",
    proof: decision("0042-decimal-has-no-size-limit.md", "decision 0042"),
  },
  {
    group: "Numbers and types",
    title: "Big numbers just work",
    what: <>A constant too big for <C>Int</C>, like <C>2 ** 100</C>, becomes a <C>BigInt</C> with the exact value. No overflow, no library.</>,
    where: "basics/data-types/bigint",
    proof: decision("0041-bigint-is-built-in.md", "decision 0041"),
  },
  {
    group: "Numbers and types",
    title: "Text to number can fail, and says so",
    what: <><C>Int(&quot;23sd&quot;)</C> is an error you must handle, never a silent <C>0</C> or a guess.</>,
    where: "basics/data-types/converting-types",
    proof: spec("02-types/01-built-in-types.md", "spec 2.1"),
  },
  {
    group: "Numbers and types",
    title: "Values print the way you write them",
    what: <><C>7.0</C> prints <C>7.0</C>, a <C>Decimal</C> keeps its scale, text inside a list is quoted. Output is never ambiguous.</>,
    where: "basics/output/how-values-print",
    proof: spec("01-basics.md", "spec 1.11"),
  },
  {
    group: "Numbers and types",
    title: "A number that can't fit is caught while compiling",
    what: <><C>const small: UInt8 = 300</C> is refused before the program runs, and the message names the type that fits: <C>UInt16</C>.</>,
    where: "basics/data-types/sized-integers",
    proof: spec("02-types/01-built-in-types.md", "spec 2.1"),
  },
  {
    group: "Fewer mistakes",
    title: "Every index is checked",
    what: <>Reading item 5 of a two-item list stops the program with the index and the list&apos;s length. It never reads memory that isn&apos;t the list&apos;s.</>,
    where: "start/reading-errors",
    proof: spec("05-errors.md", "spec 5.6"),
  },
  {
    group: "Fewer mistakes",
    title: "Asking for a number asks until it gets one",
    what: <><C>term.prompt_int</C> answers &quot;thirty&quot; with <C>Please type a whole number.</C> and asks again, so your code only ever sees a number.</>,
    where: "basics/user-input/ask-for-a-number",
    proof: spec("14-standard-library/13-term.md", "spec 14.13"),
  },
  {
    group: "Fewer mistakes",
    title: "A forgotten result is caught",
    what: <>Calling a function and ignoring what it gives back is an error, the classic bug of making an updated copy and never using it. <C>const _ =</C> says &quot;on purpose&quot;.</>,
    where: "functions/using-results",
    proof: spec("03-functions.md", "spec 3.7"),
  },
  {
    group: "Safe by default",
    title: "There is no null",
    what: <>A value that may be missing says so in its type, <C>User?</C>, and the compiler makes you handle the missing case. A null pointer error can&apos;t happen.</>,
    where: "types/optionals",
    proof: spec("02-types/07-optionals.md", "spec 2.7"),
  },
  // ---------- Clear messages ----------
  {
    group: "Clear messages",
    title: "Typos get a suggestion",
    what: <>Using an unknown name is an error that suggests the name you probably meant: <C>did you mean total?</C></>,
    where: "start/reading-errors",
    proof: spec("01-basics.md", "spec 1.4"),
  },
  {
    group: "Clear messages",
    title: "Warnings on comparisons that rarely mean what they say",
    what: <><C>==</C> between computed floats, and <C>x % 2 == 1</C> (which misses negative odd numbers), get a warning with the fix.</>,
    where: "basics/operators/comparison",
    proof: spec("01-basics.md", "spec 1.8"),
  },
  {
    group: "Clear messages",
    title: "debug() shows where, and never ships",
    what: <><C>debug(x)</C> prints the value with its file and line. Release builds leave it out, argument and all.</>,
    where: "basics/output/debug",
    proof: spec("01-basics.md", "spec 1.10"),
  },
  {
    group: "Clear messages",
    title: "One naming style, fixed by the formatter",
    what: <><C>kavach fmt</C> renames a badly styled name and every use of it, so every codebase reads the same.</>,
    where: "basics/syntax/names",
    proof: spec("01-basics.md", "spec 1.5"),
  },
  {
    group: "Clear messages",
    title: "Unused names are flagged",
    what: <>A variable nobody reads gets a warning; start its name with <C>_</C> to say it is on purpose.</>,
    where: "basics/variables",
    proof: spec("01-basics.md", "spec 1.4"),
  },
  // ---------- Fast without effort ----------
  {
    group: "Fast without effort",
    title: "A value nobody reads is never computed",
    what: <>Native code skips work whose result is unused and changes nothing you could see.</>,
    where: "basics/variables",
    proof: spec("01-basics.md", "spec 1.4"),
  },
  {
    group: "Fast without effort",
    title: "Big loops use every core",
    what: <>A loop over a big list is split across all cores when the result is exactly the one-core result, overflow checks included.</>,
    where: "speed/loops-on-all-cores",
    proof: decision("0031-loops-on-all-cores.md", "decision 0031"),
  },
  {
    group: "Fast without effort",
    title: "Chains become one loop",
    what: <><C>xs.map(f).filter(g).sum()</C> runs as a single loop: no list is built between the steps.</>,
    where: "collections/lists/map-filter-sum",
    proof: { file: "compiler/kir/src/fuse/hoist.rs", label: "fuse/hoist.rs" },
  },
  {
    group: "Fast without effort",
    title: "Small numbers are packed",
    what: <>A list of bytes uses one byte per item, and lists of 16- and 32-bit numbers are packed too.</>,
    where: "speed/byte-packed-lists",
    proof: decision("0045-packed-lists-of-small-numbers.md", "decision 0045"),
  },
  {
    group: "Fast without effort",
    title: "Small records live in registers",
    what: <>A record nobody else holds needn&apos;t be built in memory at all: its fields can stay in CPU registers.</>,
    where: "types/records",
    proof: { file: "compiler/kir/src/sroa.rs", label: "sroa.rs" },
  },
  {
    group: "Fast without effort",
    title: "Adding recursion becomes a loop",
    what: <>A function that calls itself to add up a result runs as a loop, as fast as one. Nothing you can see changes: a recursion too deep for the stack still stops with the same message.</>,
    where: "functions/recursion",
    proof: { file: "compiler/kir/src/unrecurse.rs", label: "unrecurse.rs" },
  },
  {
    group: "Fast without effort",
    title: "A stored callback is called directly",
    what: <>A function kept in a record and called later is inlined when the compiler can see which one it is.</>,
    where: "functions/functions-as-values",
    proof: { file: "compiler/kir/src/devirt.rs", label: "devirt.rs" },
  },
  {
    group: "Fast without effort",
    title: "Random reads are fetched ahead",
    what: <>In a loop reading a big list or map at computed places, the next reads are requested from memory early.</>,
    where: "speed/as-data-grows",
    proof: { file: "compiler/kir/src/prefetch.rs", label: "prefetch.rs" },
  },
  {
    group: "Fast without effort",
    title: "The GPU only when it wins",
    what: <>A <C>map</C> goes to the GPU only when the list is long and the work per item big enough; otherwise the CPU does it.</>,
    where: "speed/gpu",
    proof: spec("15-gpu.md", "spec 15"),
  },
  {
    group: "Fast without effort",
    title: "A waiting task costs nothing",
    what: <>A task waiting for the network sleeps on the operating system&apos;s event queue, and only the thread that has work wakes up.</>,
    where: "networks/sockets",
    proof: decision("0046-io-event-loop-through-mio.md", "decision 0046"),
  },
  {
    group: "Fast without effort",
    title: "You pay only for what you use",
    what: <>An executable contains only the modules, functions and runtime parts the program really uses.</>,
    where: "speed/native-executables",
    proof: decision("0024-pay-only-for-what-you-use.md", "decision 0024"),
  },
  {
    group: "Fast without effort",
    title: "Memory is freed at once",
    what: <>No garbage collector and no pauses: a value is freed the moment nothing uses it, at a point you can predict.</>,
    where: "memory/automatic-memory",
    proof: decision("0003-memory-model.md", "decision 0003"),
  },
  // ---------- Safe by default ----------
  {
    group: "Safe by default",
    title: "Copies are safe, and cheap",
    what: <>Assigning a list gives an independent value, so changing one never changes another. It is only really copied if one of them changes.</>,
    where: "memory/values-not-references",
    proof: spec("06-memory.md", "spec 6.1"),
  },
  {
    group: "Safe by default",
    title: "Failure is part of the type",
    what: <>A function that can fail says so in its signature, and the caller must deal with it. No exceptions flying past.</>,
    where: "errors/errors",
    proof: decision("0007-errors-as-values.md", "decision 0007"),
  },
  {
    group: "Safe by default",
    title: "Tests write themselves",
    what: <><C>kavach test</C> tests a function from its types and contracts alone; a failure comes with the seed to replay it.</>,
    where: "errors/testing",
    proof: decision("0010-tests-from-contracts.md", "decision 0010"),
  },
  {
    group: "Safe by default",
    title: "Code can only touch what it declares",
    what: <>A function that reads files or the network says so with <C>uses</C>; the compiler and the runtime hold it to that.</>,
    where: "agents/capabilities",
    proof: decision("0008-capabilities-in-signatures.md", "decision 0008"),
  },
  {
    group: "Safe by default",
    title: "Look-alike names are flagged",
    what: <>A name mixing letters from two scripts (a Latin <C>a</C> and a Cyrillic <C>а</C>) gets a warning, so hidden look-alikes can&apos;t sneak in.</>,
    where: "basics/syntax/names",
    proof: spec("01-basics.md", "spec 1.5"),
  },
  {
    group: "Safe by default",
    title: "Maps can't be attacked through their keys",
    what: <>Map hashing is seeded fresh for every run, so nobody can send keys built to make your maps slow.</>,
    where: "collections/maps",
    proof: { file: "docs/architecture/hash-maps.md", label: "hash-maps.md" },
  },
  {
    group: "Safe by default",
    title: "No data races",
    what: <>Tasks can&apos;t change the same value at the same time: the compiler refuses code that could.</>,
    where: "concurrency/no-data-races",
    proof: spec("09-concurrency.md", "spec 9.5"),
  },
  {
    group: "Safe by default",
    title: "Tasks take turns in a fixed order",
    what: <>The order in which tasks take turns is defined, so a concurrent program prints the same thing on every run, and a bug shows up every time.</>,
    where: "concurrency/taking-turns",
    proof: decision("0020-tasks-take-turns.md", "decision 0020"),
  },
  {
    group: "Safe by default",
    title: "Agents survive a crash",
    what: <>A durable agent writes a journal and, after a crash, replays it to carry on where it stopped, without doing anything twice.</>,
    where: "agents/durable-agents",
    proof: decision("0025-agents-resume-by-replaying-a-journal.md", "decision 0025"),
  },
  {
    group: "Safe by default",
    title: "Agent rules the model can't talk its way past",
    what: <>What an agent may touch is checked when compiling, and its limits and approvals on every call, outside anything the AI model can influence.</>,
    where: "agents/agents",
    proof: spec("08-capabilities-and-agents/03-agents.md", "spec 8.3"),
  },
  {
    group: "Safe by default",
    title: "AI answers are typed, not text",
    what: <><C>ask&lt;T&gt;</C> checks the model&apos;s answer against the type <C>T</C> and asks again when it doesn&apos;t fit, so code never parses loose text.</>,
    where: "agents/ask",
    proof: spec("08-capabilities-and-agents/04-typed-ai-calls.md", "spec 8.4"),
  },
  {
    group: "Safe by default",
    title: "Every agent call is audited",
    what: <>Each capability call an agent makes is written down with its verdict and result, refused ones too, without a line of logging code.</>,
    where: "agents/audit-log",
    proof: spec("08-capabilities-and-agents/06-audit-log.md", "spec 8.6"),
  },
  {
    group: "Safe by default",
    title: "Secrets stay secret",
    what: <>A secret prints as <C>&lt;secret&gt;</C> and can&apos;t reach logs or messages unless you reveal it on purpose.</>,
    where: "agents/secrets",
    proof: spec("08-capabilities-and-agents/07-secrets.md", "spec 8.7"),
  },
];
