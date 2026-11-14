import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { Limits } from "./limits";
import { ModuleTable } from "./spec-table";
import { fileFigure, specBlock } from "@/lib/spec-tables";
import { SrcRef } from "@/components/src-ref";

// ---------- The Terminal ----------

export const termDoc: LessonDoc = {
  lead: "std.term colors and styles text, draws progress bars, and asks questions. It turns its colors off by itself when output isn't a terminal.",
  simple: <><C>std.term</C> makes what a program prints easier to read: red for an error, green for done, a bar that fills up as a job goes on. When the output goes to a file instead, it leaves the colors out, so the file stays clean.</>,
  minutes: 4,
  before: ["basics/output", "basics/user-input"],
  body: (
    <>
      <Ex name="term/colors" />
      <P>
        In a terminal, <C>error:</C> is red and <C>done</C> green and bold. The output above has no colors because
        this page captured it through a pipe, not a terminal, and <C>std.term</C> never writes color codes there: log
        files and pipes stay clean. Setting the environment variable <C>NO_COLOR</C> turns colors off too.
      </P>
      <H2>Progress bars</H2>
      <Ex name="term/progress" />
      <P>
        In a terminal, the bar is drawn on one line and updated as the job goes on:{" "}
        <C>[############--------]  60%  6/10</C>. Outside one, as here, it prints nothing, so a log or a pipe gets no
        half-drawn lines.
      </P>
      <ModuleTable file="13-term.md" number="14.13" first="Function or method" heading="Progress bars" />
    </>
  ),
};

export const colorsDoc: LessonDoc = {
  lead: "Each color or style function takes text and gives it back colored. They nest: an inner style ends only itself.",
  simple: <>Like coloured pens: write a sentence in red, switch to green for one word, and the rest of the sentence goes back to red. Each style covers only its own text.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="term/colors" />
      <ModuleTable file="13-term.md" number="14.13" first="Function" heading="" />
      <P>In <C>term.red(&quot;a&quot; + term.green(&quot;b&quot;) + &quot;c&quot;)</C>, <C>c</C> is red again: each function resets only its own style.</P>
    </>
  ),
};

export const promptsDoc: LessonDoc = {
  lead: "term.prompt, prompt_int, confirm and choose ask the user a question and give a typed answer.",
  simple: <>The program asks a question and waits for the person to answer, like a shop assistant asking &quot;Anything else?&quot;. When it asks for a number, you get a number back, not just letters.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="term/confirm" />
      <ModuleTable file="13-term.md" number="14.13" first="Function" heading="Prompts" />
      <P>
        Prompts read standard input whether or not it is a terminal, so a script can answer them through a pipe, as
        this page did. See <Link href="/docs/basics/user-input/">User Input</Link> for names, numbers and passwords.
      </P>
    </>
  ),
};

// ---------- Time ----------

const CONFORMANCE_14 = "tests/conformance/14-standard-library";
/** How far from 1970 a zone's `at` and `iso` take a moment, as their contract says. */
const zoneRange = () => fileFigure("stdlib/time/time.kav", /pub fn iso\(self, ms: Int\) -> String\s+require ms >= -([\d_]+) and/);

export const timeDoc: LessonDoc = {
  lead: "std.time tells the time, measures how long something takes, waits, repeats work at an interval, works with dates and time zones, and reads and writes times as text.",
  simple: <>A watch, a stopwatch and a calendar in one: read the time, measure how long a job took, wait a moment, work out the date a month from now, see what the clocks show in another city, or read a date and time out of a line of text.</>,
  minutes: 8,
  before: ["basics/for-loop"],
  body: (
    <>
      <Ex name="time/measure" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>time.now_ms()</C> is the time of day, in milliseconds since 1970. The clock can be changed, so don&apos;t measure with it.</li>
        <li><C>time.monotonic_ms()</C> never goes back: subtract two readings to measure a duration.</li>
      </ul>
      <H2>Every few moments</H2>
      <Ex name="time/every" />
      <P>Each round starts 100 ms after the one before. A round that takes longer makes the next start at once: rounds are never skipped or doubled.</P>
      <H2>Durations</H2>
      <Ex name="time/durations" />
      <ModuleTable file="04-time.md" number="14.4" first="Written" />
      <P>
        A <C>Duration</C> is a number of milliseconds, so <C>1.days</C> is always exactly 24 hours: daylight saving and
        time zones don&apos;t change it. There is no <C>.months</C>, because months differ in length: for those, use
        a date&apos;s <C>add_months</C>, below. Writing one is refused, with what to write instead:
      </P>
      <Ex name="time/months.fails" />
      <H2>Every function</H2>
      <ModuleTable file="04-time.md" number="14.4" first="Function" />
      <Note>
        Reading the clock needs no capability: it changes nothing outside the program. Waiting, with{" "}
        <C>sleep</C>, <C>every_ms</C> or a timer, needs <C>Clock</C>. In the tests <C>kavach test</C> writes for
        you, a fake clock makes reading the time, <C>sleep</C>, <C>after</C> and each round of <C>every</C> instant
        and the same every time. A <C>test</C> block sets one with <C>time.freeze</C>, below.
      </Note>
      <H2>When the clock changes</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          Waits (<C>sleep</C>, <C>every</C>, <C>after</C>, timeouts) and <C>monotonic_ms</C> use the system&apos;s
          clock that never jumps, as Rust, Go, Java and Python do: changing the clock or the time zone doesn&apos;t
          move them.
        </li>
        <li>Whether time the machine slept counts is the system&apos;s own rule: it doesn&apos;t on Linux and macOS, and it does on Windows.</li>
        <li>
          There is no wait until a time of day, and no cron schedule. A job at 09:00 every day belongs to the
          system&apos;s scheduler (cron, systemd timers, Task Scheduler), which starts the program.
        </li>
      </ul>
      <H2>The clock in tests</H2>
      <P>
        A <C>test</C> block that needs a date calls <C>time.freeze(ms)</C>. From then on the clock is fake:{" "}
        <C>now_ms</C> gives <C>ms</C>, and <C>sleep</C> moves it on at once, so 30 days pass in no time:
      </P>
      <Ex name="time/freeze.test" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          Calling it again jumps the clock forward or back, as when a person changes the machine&apos;s clock.{" "}
          <C>monotonic_ms</C> doesn&apos;t jump: it counts only what the test slept.
        </li>
        <li>The tests <C>kavach test</C> writes for you, and every <C>example</C>, need no call: their clock is already frozen at 2026-01-01 00:00 UTC. That is the moment the example above starts its trial.</li>
        <li>Anywhere but directly in a <C>test</C> block, it is an error:</li>
      </ul>
      <Ex name="time/freeze-outside.fails" />
      <H2>Dates</H2>
      <ModuleTable file="04-time.md" number="14.4" first="Type" />
      <Ex name="time/dates" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Months are 1 to 12</strong>, never from 0 as in JavaScript.</li>
        <li><strong className="text-ink">A month too short for the day gives its last day:</strong> January 31 and one month is February 29 in 2024, a leap year, and February 28 in 2026.</li>
        <li><strong className="text-ink">Every year uses the Gregorian calendar:</strong> 1900 is not a leap year, 2000 is.</li>
        <li><strong className="text-ink">A date that can&apos;t exist is refused.</strong> <C>parse_date</C> fails with <C>Invalid</C>, and making one stops the program:</li>
      </ul>
      <Ex name="time/impossible.panics" />
      <P>
        A <C>DateTime</C> has no time zone: it is what a calendar and a wall clock show. A moment, milliseconds since
        1970 as <C>now_ms</C> gives it, is another thing, so the two can&apos;t be mixed by accident.{" "}
        <C>time.utc(ms)</C> turns a moment into its <C>DateTime</C> in UTC, and <C>dt.utc_ms()</C> back. A named time
        zone, below, does the same for any place.
      </P>
      <ModuleTable file="04-time.md" number="14.4" first="Function or method" heading="Dates and times of day" />
      <H2>Time zones</H2>
      <P>
        A <C>Zone</C> is a named time zone, such as <C>Asia/Kolkata</C> or <C>America/New_York</C>: what the clocks
        there show at each moment, daylight saving time included. Like reading the clock, it needs no capability.
        One moment, in three places:
      </P>
      <Ex name="time/zones" />
      <ModuleTable file="04-time.md" number="14.4" first="Function or method" heading="Time zones" />
      <P>
        When daylight saving time starts, the clocks skip an hour, so some times of day never happen there.{" "}
        <C>z.moment(dt)</C> moves a skipped time later by the gap, as iCalendar, Java and JavaScript&apos;s Temporal
        do. To choose otherwise, match on <C>z.resolve(dt)</C>:
      </P>
      <Ex name="time/zone-skipped" />
      <P>When it ends, the clocks go back, so some times happen twice. <C>z.moment(dt)</C> takes the first:</P>
      <Ex name="time/zone-repeated" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>
          <strong className="text-ink">Kavach never reads the machine&apos;s own time zone.</strong> A program names
          the zone it means, so it runs the same on every machine.
        </li>
        <li>The zone rules come from the operating system on Linux and macOS, and from a copy built into the program on Windows.</li>
        <li>An unknown name fails with <C>UnknownZone</C>, and <C>time.zone_offset</C> gives <C>none</C>:</li>
      </ul>
      <Ex name="time/zone-unknown" />
      <H2>A fixed offset</H2>
      <P>
        A <C>Zone</C> can also be a fixed offset from UTC, such as <C>+05:30</C>, which never changes.{" "}
        <C>z.iso(ms)</C> writes a moment as the clocks in the zone show it, with the offset:
      </P>
      <Ex name="time/zone-fixed" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An offset of zero is written <C>Z</C>.</li>
        <li>An offset has seconds only when it needs them, as some zones&apos; oldest offsets do.</li>
        <li>An offset that can&apos;t be fails with <C>UnknownZone</C>, like an unknown name.</li>
        <li><C>z.at(ms)</C> and <C>z.iso(ms)</C> take a moment within ±<C>{zoneRange()}</C> ms of 1970, as their contract says.</li>
      </ul>
      <H2>Times as text</H2>
      <P>
        <C>time.parse_date</C> and three more read the formats <C>iso</C> writes. A <C>Time</C> and a{" "}
        <C>DateTime</C> have no zone, so their text can&apos;t have one. A moment needs a zone, so its text must:
      </P>
      <ModuleTable file="04-time.md" number="14.4" first="Function" heading="Times as text" />
      <Ex name="time/text" />
      <P>Each of them fails with <C>Invalid(text)</C> for anything else:</P>
      <Ex name="time/text-refused" />
      <Ex name="time/expanded-years" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Numbers have exactly the digits shown, so <C>9:05</C> and <C>2026-11-4</C> fail, and so does a sign where a digit should be.</li>
        <li>A year before 0 or after 9999 is written as <C>iso</C> writes it, with a sign or more digits, as ISO 8601&apos;s expanded years are: <C>-0005-01-01</C>, <C>12345-06-07</C> or <C>+12345-06-07</C>.</li>
        <li>A fraction of a second has 1 to 9 digits. Kavach keeps milliseconds, so digits past the third are dropped, not rounded: <C>.1239</C> is 123 ms.</li>
        <li>A leap second (<C>23:59:60</C>) fails: a <C>Time</C> has no such second.</li>
        <li>In JSON, a <C>Date</C>, <C>Time</C> and <C>DateTime</C> are this text too, with a <C>DateTime</C> in UTC: see <Link href="/docs/standard-library/json/decoding-into-a-type/#dates-and-times">Dates and times</Link>.</li>
        <li><C>parse_moment</C> reads back any moment <C>z.iso</C> wrote, seconds in the offset included.</li>
        <li>Other formats, such as <C>4 Nov 2026</C>, are not read: they are an open question.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/04-time.md" label="spec 14.4 · times as text" />
        <SrcRef file={`${CONFORMANCE_14}/times-are-read-from-text.kav`} label="times from text, tested" />
        <SrcRef file={`${CONFORMANCE_14}/moments-are-read-only-with-an-offset.kav`} label="moments from text, tested" />
        <SrcRef file={`${CONFORMANCE_14}/zones-write-moments-with-their-offset.kav`} label="writing with an offset, tested" />
        <SrcRef file={`${CONFORMANCE_14}/zones-can-be-fixed-offsets.kav`} label="fixed offsets, tested" />
      </div>
      <H2>The time in a contract</H2>
      <P>Reading the clock needs no capability, but a contract may not do it: its answer would change from run to run.</P>
      <Ex name="time/clock-contract.fails" />
    </>
  ),
};

// ---------- Random Numbers ----------

export const randomDoc: LessonDoc = {
  lead: "std.random gives random whole numbers, picks and shuffles. Its numbers are safe for passwords and keys, unless a block asks for repeatable ones from a seed.",
  simple: <>Like rolling dice or shuffling a deck of cards, inside the program. The results are hard enough to guess that you can use them for passwords.</>,
  minutes: 3,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="random/dice" />
      <P>
        <C>random.int(1, 7)</C> gives 1 to 6: the end is left out, as in <C>1..7</C>. The program checks its own rolls,
        because random output can&apos;t be written down in advance.
      </P>
      <Ex name="random/pick" />
      <ModuleTable file="15-random.md" number="14.15" first="Function" />
      <Note>
        Every function needs <C>uses Random</C>. Under <C>kavach test</C>, random numbers come from the test
        run&apos;s seed instead, so a failing run can be repeated exactly.
      </Note>
      <H2>Seeded</H2>
      <P>
        <C>random.seeded(seed) {"{ … }"}</C> runs its block with every <C>std.random</C> function drawing from{" "}
        <C>seed</C> instead of the system. The same seed gives the same numbers, on every machine and in every version
        of Kavach: for simulations, generated worlds and games whose players must all see the same thing.
      </P>
      <Ex name="random/seeded" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>These numbers are not safe for keys, tokens or passwords: anyone who knows the seed knows the numbers.</li>
        <li><C>uses Random</C> is still needed. Under <C>kavach test</C>, the seed wins over the test run&apos;s.</li>
        <li>Seeds nest: an inner <C>seeded</C> block draws from its own seed, and the outer one goes on where it left off.</li>
        <li>The seed belongs to the task running the block. A task spawned inside it draws from the system, since tasks may take turns in different orders on different machines.</li>
        <li>The numbers come from xoshiro256**, its state made from the seed by SplitMix64, as their published versions make it.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/15-random.md" label="spec 14.15 · seeded" />
        <SrcRef file="tests/conformance/14-standard-library/a-seed-gives-the-same-numbers.kav" label="tested" />
      </div>
      <H2>Limits and sizes</H2>
      <Limits id="random" />
    </>
  ),
};

// ---------- Clipboard ----------

export const clipboardDoc: LessonDoc = {
  lead: "std.clipboard reads and writes the system clipboard's text. What a person copied can be private, so it needs the capability Clipboard.",
  simple: <>The clipboard is the invisible note that copy and paste use. A program may read or replace that note only when it says so, because people copy passwords and messages too.</>,
  minutes: 2,
  before: ["agents/capabilities"],
  body: (
    <>
      <CodeBlock code={specBlock("14-standard-library/17-clipboard.md", "14.17")} file="main.kav, from spec 14.17" />
      <ModuleTable file="17-clipboard.md" number="14.17" first="Function" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>ClipboardError</C> is <C>Empty</C>, or <C>Unavailable(reason)</C> on a machine with no clipboard: a server, a container, a terminal with no desktop.</li>
        <li>On Linux with X11, text a program writes stays on the clipboard only while the program runs; a clipboard manager, which most desktops have, keeps it after.</li>
        <li>Text only, for now: images and other kinds of content are an open question.</li>
      </ul>
      <H2>It needs Clipboard</H2>
      <P>A function that reads or writes the clipboard says so in its <C>uses</C>, or it doesn&apos;t compile:</P>
      <Ex name="clipboard/needs.fails" />
      <H2>In tests</H2>
      <P>
        Under <C>kavach test</C> the clipboard is a fake in memory that starts empty, so a test never reads or
        changes the real one:
      </P>
      <Ex name="clipboard/copy.test" />
    </>
  ),
};

// ---------- Devices ----------

export const devicesDoc: LessonDoc = {
  lead: "std.devices talks to devices on a cable. Serial ports come first: boards, sensors and modems on USB or RS-232. They need the capability Serial.",
  simple: <>A serial port is a cable that sends a device one letter after another, like a very fast telegraph. Kavach lets a program talk to a board or a sensor this way, but only a program that says it will.</>,
  minutes: 3,
  before: ["standard-library/bytes"],
  body: (
    <>
      <CodeBlock code={specBlock("14-standard-library/18-devices.md", "14.18")} file="main.kav, from spec 14.18" />
      <P>
        It lists the ports, opens one at 115,200 bits a second, sends <C>ping</C> and prints the answer. A port is
        named as the system names it: <C>COM3</C> on Windows, <C>/dev/ttyUSB0</C> on Linux.
      </P>
      <ModuleTable file="18-devices.md" number="14.18" first="Function or method" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A port runs with 8 data bits, no parity and 1 stop bit, the setting most devices use.</li>
        <li><C>read</C> waits at most a second for the first byte, and gives empty <C>Bytes</C> when none came. While it waits, the task keeps its turn.</li>
        <li>Each kind of device has a capability of its own, so a function&apos;s signature shows what it can reach, and an agent gets one only when a person approves it.</li>
      </ul>
      <H2>When a port isn&apos;t there</H2>
      <P>
        <C>DeviceError</C> is <C>NotFound(path)</C>, <C>Denied(path)</C> (another program has the port, or the user
        may not open it) or <C>Failed(path, reason)</C>:
      </P>
      <Ex name="devices/not-found" />
      <H2>It needs Serial</H2>
      <P>A function that reaches a port says so in its <C>uses</C>, or it doesn&apos;t compile:</P>
      <Ex name="devices/needs.fails" />
      <Note>
        <C>kavach test</C> has no fake serial port yet, so a function that uses <C>Serial</C> gets no generated
        tests.
      </Note>
    </>
  ),
};

// ---------- Regex ----------

export const regexDoc: LessonDoc = {
  lead: "A regular expression is a pattern that text can match. regex.new makes one; its methods find, capture, replace and split.",
  simple: <>A pattern is a description of what you look for, such as &quot;four digits, a dash, then two digits&quot;. Instead of searching for one exact word, you find every piece of text that fits the description.</>,
  minutes: 5,
  before: ["basics/strings"],
  body: (
    <>
      <Ex name="regex/match" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Write patterns as raw strings, <C>r&quot;…&quot;</C>, so <C>\d</C> needs no extra backslash.</li>
        <li><C>(?&lt;year&gt;…)</C> is a named group; <C>named_captures</C> gives each one by name.</li>
      </ul>
      <H2>Find, split, replace</H2>
      <Ex name="regex/find" />
      <Ex name="regex/replace" />
      <P>In the replacement, <C>${"{year}"}</C> and <C>$1</C> insert a group: a raw string keeps <C>{"{…}"}</C> from being read as interpolation.</P>
      <H2>A pattern that is wrong</H2>
      <Ex name="regex/bad" />
      <H2>Every method</H2>
      <ModuleTable file="08-regex.md" number="14.8" first="Method" />
      <Note>
        <C>regex.new</C> matches in time proportional to the text, whatever the pattern, so text from users can&apos;t
        make it hang. <C>regex.advanced</C> also allows lookaround and backreferences, and gives up after a fixed
        amount of work.
      </Note>
    </>
  ),
};
