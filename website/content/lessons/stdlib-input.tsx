import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Gamepads ----------
//
// Spec 14.26. Reading a real pad shows what is plugged into this machine,
// and rumble shakes it, so the samples read no pad: they name the buttons
// and test a game's logic with a Pad made by hand.

const FILE = "14-standard-library/26-input.md";
const TESTS = "tests/conformance/14-standard-library";

export const gamepadDoc: LessonDoc = {
  lead: "std.input reads gamepads in one standard layout, whatever pad is plugged in: as state a game loop reads once a frame, or as events in a window's stream. It needs Input, as the keyboard does.",
  simple: <>A gamepad is the controller you hold to play a game: buttons, two sticks and two triggers. Every brand prints different letters on its buttons, so Kavach names each button by where it sits, and a game works with any pad.</>,
  minutes: 5,
  before: ["standard-library/windows", "agents/capabilities"],
  body: (
    <>
      <P>
        A game loop asks for the pads once a frame and reads each one. This is the spec&apos;s example. It reads the
        pads plugged into the machine, and shakes one, so it is shown here, not run:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.26", "fn steer(")} file="from spec 14.26" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>input.pads()</C> gives each pad connected now, as a <C>Pad</C>. The first call starts watching them.</li>
        <li><C>pad.is_down(&quot;South&quot;)</C> says whether a button is held.</li>
        <li><C>pad.left.x</C> is where the left stick is, from -1 (left) to 1 (right); <C>y</C> goes from -1 (down) to 1 (up).</li>
        <li><C>pad.rumble(strong, weak, millis)</C> shakes the pad: its strong (low) motor and its weak (high) one, each from 0 to 1, for up to 60,000 milliseconds.</li>
      </ul>
      <H2>Buttons are named by their place</H2>
      <P>
        <C>South</C> is the bottom button: A on an Xbox pad, Cross on a PlayStation pad. So a game says{" "}
        <C>&quot;South&quot;</C> once and works on both. <C>input.buttons()</C> gives every name, and needs nothing:
      </P>
      <Ex name="gamepads/buttons" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>LeftStick</C> and <C>RightStick</C> are the sticks pressed in.</li>
        <li>A trigger is a button and an axis: <C>LeftTrigger</C> is held once it is pressed far enough, and <C>left_trigger</C> says how far.</li>
        <li>A small movement around a stick&apos;s middle counts as none, so a pad left alone reads 0.</li>
      </ul>
      <H2>A pad is a record</H2>
      <P>
        So a game&apos;s logic can be tested with a pad made by hand, with no pad plugged in. A function that only reads
        a <C>Pad</C> needs no capability:
      </P>
      <Ex name="gamepads/made-up-pad" />
      <ModuleTable file="26-input.md" number="14.26" first="Field" />
      <H2>The gyroscope</H2>
      <P>
        Many pads know how fast they turn. <C>pad.gyro</C> gives it in radians a second, or <C>none</C> for a pad with
        no gyroscope. A game reads it once a frame and multiplies it by the frame&apos;s time, to aim or turn its
        camera by moving the pad:
      </P>
      <Ex name="gamepads/gyro" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>x</C> is pitch (the far edge up), <C>y</C> yaw (turning left), <C>z</C> roll (the right side down).</li>
        <li>It is state only, with no events, which would come a thousand times a second.</li>
        <li><C>pad.gyro?.y ?? 0.0</C> reads one axis and gives 0 for a pad without one.</li>
        <li>A <C>Pad</C> made by hand, as in a test, has <C>gyro: none</C> unless it is given one, as <C>old</C> is.</li>
      </ul>
      <H2>Reading pads needs Input</H2>
      <P>
        A pad is input, like the keyboard. A private function may leave its <C>uses</C> out, and the compiler works it
        out. A <C>pub</C> one must say it:
      </P>
      <Ex name="gamepads/needs-input.fails" />
      <H2>Events in a window</H2>
      <P>
        Every window that listens to input gets gamepad events too: <C>PadConnected</C>, <C>PadGone</C>,{" "}
        <C>PadButton</C> and <C>PadAxis</C>. They are in the table of events on{" "}
        <Link href="/docs/standard-library/windows/">Windows</Link>. An event tells of each press, even one that ends
        between two frames; <C>input.pads()</C> says where everything is now.
      </P>
      <Note>
        A machine where gamepads can&apos;t be read, such as a container with no udev on Linux, has no pads:{" "}
        <C>input.pads()</C> is empty and no pad events come. That is not an error.
      </Note>
      <H2>Every function and method</H2>
      <ModuleTable file="26-input.md" number="14.26" first="Function or method" />
      <H2>Not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Accelerometers, the touchpads of some pads, and a pad&apos;s battery. Touch screens are <Link href="/docs/standard-library/windows/">window events</Link>.</li>
        <li>Mapping a pad the standard layout doesn&apos;t know, from the program.</li>
        <li>Generated tests skip functions that use <C>Input</C>. See <Link href="/docs/errors/testing/">Testing</Link>.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.26 · std.input" />
        <SrcRef file={`${TESTS}/gamepads-are-named-by-place.kav`} label="buttons, tested" />
        <SrcRef file={`${TESTS}/gamepads-need-input.kav`} label="Input, tested" />
        <SrcRef file="stdlib/input/input.kav" label="input.kav" />
        <SrcRef file="runtime/input/src/lib.rs" label="runtime/input" />
        <SrcRef file="docs/decisions/0124-gamepads-through-sdl3.md" label="decision 0124" />
      </div>
    </>
  ),
};
