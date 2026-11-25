import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P, SpecCode } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Windows ----------
//
// The examples open real windows, so this site shows them from the spec
// and runs only the ones the compiler refuses.

const FILE = "14-standard-library/22-window.md";

export const windowDoc: LessonDoc = {
  lead: "std.window opens windows on the screen and reads the keyboard, mouse, touch and gamepad input that comes to them: the start of games, tools, charts and user interfaces.",
  simple: <>A window is a frame on the screen that a program owns. Kavach lets a program open one, and hear what a person does in it, like a key press or a mouse click, but only when the program says so.</>,
  minutes: 4,
  before: ["agents/capabilities"],
  body: (
    <>
      <SpecCode file={FILE} number="14.22" text={'window.open("Swarm"'} name="main.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>window.open</C> shows a window of 1280 × 720 pixels at once.</li>
        <li><C>events()</C> waits for at least one event, then gives every event that came.</li>
        <li><C>Closed</C> means the person asked to close it. The window stays open until the program closes it or lets go of it, so it can ask &quot;save first?&quot;.</li>
        <li>On a machine that shows no windows, such as a server with no screen, <C>open</C> fails with <C>WindowError.Unavailable(reason)</C>.</li>
      </ul>
      <H2>Two capabilities</H2>
      <P>
        <C>Window</C> opens windows, changes them and reads their window events. <C>Input</C> reads the keyboard, the
        mouse, typed text and gamepads. So a function that only draws can&apos;t read what a person types:
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
      <SpecCode file={FILE} number="14.22" text={"win.run("} />
      <P>
        The system&apos;s event loop runs on a thread of the runtime&apos;s own, so a slow frame never freezes the
        window.
      </P>
      <H2>Every function and method</H2>
      <ModuleTable file="22-window.md" number="14.22" first="Function or method" />
      <H2>Events</H2>
      <P>
        <C>Event</C> is a union. The variants from <C>Key</C> on come only after <C>listen_input()</C>, which
        needs <C>Input</C>. Gamepad events come to every window that listens to input; see{" "}
        <Link href="/docs/standard-library/gamepads/">Gamepads</Link>:
      </P>
      <ModuleTable file="22-window.md" number="14.22" first="`Event`" />
      <Note>
        <C>Key</C> names a key&apos;s place, not its letter: <C>KeyW</C> is the same key on every keyboard layout, as a
        game wants. For text a person types, read <C>Text</C>, which follows their layout.
      </Note>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A <C>Touch</C> is a finger or a pen on a touch screen. Each one keeps its <C>id</C> from <C>Began</C>, through <C>Moved</C>, to <C>Ended</C>, so a program tells two fingers apart.</li>
        <li><C>Cancelled</C> means the system took the touch away, as for a gesture of its own: undo what the touch started.</li>
        <li><C>pressure</C> is from 0 to 1. A screen that doesn&apos;t measure it gives 1.</li>
      </ul>
      <H2>Waking and drawing again</H2>
      <P>
        A program that only answers events sleeps in <C>events()</C> until something happens, so it uses no power
        while nothing does. Other tasks run while it waits. When one of them has something to show, it calls{" "}
        <C>win.wake()</C>, and the waiting <C>events()</C> gives <C>Woken</C>:
      </P>
      <SpecCode file={FILE} number="14.22.1" text={'window.open("Clock"'} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>wake()</C> may be called from any task, even one on another core. On a closed window it does nothing.</li>
        <li>Several wakes before the events are taken give one <C>Woken</C>, and several <C>Redraw</C>s give one: each means &quot;look again&quot;, not how many times.</li>
        <li><C>Redraw</C> comes when the system needs the window&apos;s picture: when it is first shown, uncovered or resized. A program that draws only when something changes draws at each <C>Redraw</C>, and after its own changes.</li>
        <li><C>Redraw</C> and <C>Woken</C> come without <C>listen_input()</C>.</li>
        <li><strong className="text-ink">Resizing on Windows.</strong> While a person drags the border, the window&apos;s events wait. A window drawn with the GPU is drawn again at each step: after each <C>Redraw</C>, the system waits up to 50 ms for the next <C>frame.show()</C>, so the picture follows the border instead of stretching.</li>
      </ul>
      <H2>Text input</H2>
      <P>
        A text box needs more than a game does: what each key means, held keys that repeat, the modifiers held,
        double clicks, and the text an input method composes before it is typed. All of it comes in the one{" "}
        <C>Event</C> stream:
      </P>
      <SpecCode file={FILE} number="14.22.2" text={"fn edit(box: TextBox, e: Event)"} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><strong className="text-ink">Places and names.</strong> <C>key</C> is the key&apos;s place, for games and shortcuts by place. <C>name</C> is what the keyboard layout makes of it, for shortcuts by letter: Ctrl+Z is the key named <C>z</C>, wherever the layout puts it. Typed text comes as <C>Text</C>, never from <C>name</C>.</li>
        <li><strong className="text-ink">Repeats.</strong> A held key gives <C>Key</C> events with <C>repeat</C> true, with no key-up between them, and a <C>Text</C> each time when it types.</li>
        <li><strong className="text-ink">Clicks.</strong> A press of the same button within 500 ms and 4 pixels of the last makes <C>clicks</C> one more: 2 is a double click, 3 a triple click.</li>
        <li><strong className="text-ink">Modifiers.</strong> <C>Modifiers</C> holds <C>shift</C>, <C>ctrl</C>, <C>alt</C> and <C>meta</C> (the Windows key, or Command on a Mac), as they were when the key, button, move or wheel came.</li>
        <li><strong className="text-ink">Input methods.</strong> Chinese, Japanese, Korean and many other languages are typed through an input method: a person types keys, picks from a list, and only then is text typed. <C>win.set_text_input(true)</C> turns it on while a text box has the focus, and <C>false</C> turns it off, so a game&apos;s keys go to the game. While a person composes, <C>Composing</C> events say what to show, underlined; <C>Text</C> gives what was typed in the end.</li>
        <li><C>win.set_caret(x, y, width, height)</C> says where the caret is, so the input method shows its list beside it. Say it again when the caret moves.</li>
      </ul>
      <P>Turning on an input method reads what a person types, so it needs <C>Input</C>:</P>
      <Ex name="window/text-input.fails" />
      <H2>Not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A pen&apos;s tilt and its buttons. Drawing into a window is done with the GPU: see <Link href="/docs/standard-library/gpu-screens/">GPU Screens</Link>.</li>
        <li>On macOS, <C>window.open</C> fails with <C>Unavailable</C> for now: macOS shows windows only from a program&apos;s first thread.</li>
        <li>Generated tests skip functions that use <C>Window</C> or <C>Input</C>: there is no fake screen yet. See <Link href="/docs/errors/testing/">Testing</Link>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.22 · std.window" />
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.22.1 · waking" />
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.22.2 · text input" />
        <SrcRef file="tests/conformance/14-standard-library/windows-open-or-say-why.kav" label="open, tested" />
        <SrcRef file="tests/conformance/14-standard-library/window-input-needs-input.kav" label="Input, tested" />
        <SrcRef file="stdlib/window/window.kav" line={90} symbol="set_text_input" />
        <SrcRef file="stdlib/window/window.kav" line={108} symbol="wake" />
        <SrcRef file="docs/decisions/0104-windows-through-winit.md" label="decision 0104" />
      </div>
    </>
  ),
};
