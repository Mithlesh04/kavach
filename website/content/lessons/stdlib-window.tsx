import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Windows ----------
//
// The examples open real windows, so this site shows them from the spec
// and runs only the ones the compiler refuses.

const FILE = "14-standard-library/22-window.md";

export const windowDoc: LessonDoc = {
  lead: "std.window opens windows on the screen and reads the keyboard and mouse input that comes to them: the start of games, tools, charts and user interfaces.",
  simple: <>A window is a frame on the screen that a program owns. Kavach lets a program open one, and hear what a person does in it, like a key press or a mouse click, but only when the program says so.</>,
  minutes: 4,
  before: ["agents/capabilities"],
  body: (
    <>
      <CodeBlock code={specBlockWith(FILE, "14.22", 'window.open("Swarm"')} file="main.kav, from spec 14.22" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>window.open</C> shows a window of 1280 × 720 pixels at once.</li>
        <li><C>events()</C> waits for at least one event, then gives every event that came.</li>
        <li><C>Closed</C> means the person asked to close it. The window stays open until the program closes it or lets go of it, so it can ask &quot;save first?&quot;.</li>
        <li>On a machine that shows no windows, such as a server with no screen, <C>open</C> fails with <C>WindowError.Unavailable(reason)</C>.</li>
      </ul>
      <H2>Two capabilities</H2>
      <P>
        <C>Window</C> opens windows, changes them and reads their window events. <C>Input</C> reads the keyboard, the
        mouse and typed text. So a function that only draws can&apos;t read what a person types:
      </P>
      <Ex name="window/input.fails" />
      <P>Opening a window needs <C>Window</C>, like every way to the outside world:</P>
      <Ex name="window/window.fails" />
      <H2>Waiting, polling or answering</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>win.events()</C> waits: for a tool that does nothing until a person acts.</li>
        <li><C>win.poll()</C> never waits, and is empty when nothing came: a game calls it once a frame.</li>
        <li><C>win.run(f)</C> calls <C>f</C> for each event until <C>f</C> gives <C>false</C> or the window closes:</li>
      </ul>
      <CodeBlock code={specBlockWith(FILE, "14.22", "win.run(")} file="from spec 14.22" />
      <P>
        The system&apos;s event loop runs on a thread of the runtime&apos;s own, so a slow frame never freezes the
        window.
      </P>
      <H2>Every function and method</H2>
      <ModuleTable file="22-window.md" number="14.22" first="Function or method" />
      <H2>Events</H2>
      <P>
        <C>Event</C> is a union. The last six variants come only after <C>listen_input()</C>, which needs{" "}
        <C>Input</C>:
      </P>
      <ModuleTable file="22-window.md" number="14.22" first="`Event`" />
      <Note>
        <C>Key</C> names a key&apos;s place, not its letter: <C>KeyW</C> is the same key on every keyboard layout, as a
        game wants. For text a person types, read <C>Text</C>, which follows their layout.
      </Note>
      <H2>Not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Drawing into a window comes with GPU rendering; gamepads and touch with a later <C>std.input</C>.</li>
        <li>On macOS, <C>window.open</C> fails with <C>Unavailable</C> for now: macOS shows windows only from a program&apos;s first thread.</li>
        <li>Generated tests skip functions that use <C>Window</C> or <C>Input</C>: there is no fake screen yet. See <Link href="/docs/errors/testing/">Testing</Link>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.22 · std.window" />
        <SrcRef file="tests/conformance/14-standard-library/windows-open-or-say-why.kav" label="open, tested" />
        <SrcRef file="tests/conformance/14-standard-library/window-input-needs-input.kav" label="Input, tested" />
        <SrcRef file="stdlib/window/window.kav" label="window.kav" />
        <SrcRef file="docs/decisions/0104-windows-through-winit.md" label="decision 0104" />
      </div>
    </>
  ),
};
