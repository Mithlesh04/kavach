import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- GPU Screens ----------
//
// Its programs open windows and draw with the GPU, so this site shows
// them from the spec and doesn't run them. Drawing with pipelines
// (frame.draw, gpu.pipeline, spec 15.3) is specified but not built yet.

const FILE = "14-standard-library/24-gpu.md";

export const gpuScreenDoc: LessonDoc = {
  lead: "std.gpu draws into a window with the GPU: a screen for the window, a frame for each picture, cleared and shown at the screen's rate. Drawing triangles with shaders written in Kavach comes next.",
  simple: <>A window is a frame on the screen; the GPU is the fast painter that fills it. Each picture is painted on a fresh canvas and then shown, many times a second, which is how games and animations move.</>,
  minutes: 4,
  before: ["standard-library/windows"],
  body: (
    <>
      <CodeBlock code={specBlockWith(FILE, "14.24", "gpu.screen(win)")} file="main.kav, from spec 14.24" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>gpu.screen(win)</C> gives what draws into the window, using the program&apos;s GPU.</li>
        <li><C>screen.frame()</C> gives the next picture, as big as the window is now. It waits for the screen&apos;s next refresh when the last picture isn&apos;t shown yet (vsync), so the loop runs at the screen&apos;s rate.</li>
        <li><C>frame.clear(r, g, b)</C> fills it with one color, each part from 0 to 1, and <C>frame.show()</C> puts it on the screen. A frame never shown is thrown away.</li>
        <li>Drawing changes the program&apos;s own windows, so it needs <C>Window</C>.</li>
        <li>Where there is no GPU, or the window can&apos;t be drawn into, it fails with <C>GpuError.Unavailable(reason)</C>. So does a frame of a minimized window; the next <C>frame()</C> tries again.</li>
      </ul>
      <P>
        A private function may leave its <C>uses</C> out, and the compiler works it out. A <C>pub</C> one must say it, so
        a function that draws for other modules names <C>Window</C>:
      </P>
      <Ex name="gpu/screen-needs-window.fails" />
      <H2>Every function and method</H2>
      <ModuleTable file="24-gpu.md" number="14.24" first="Function or method" />
      <Note>
        Built now: <C>gpu.screen</C>, <C>frame</C>, <C>clear</C> and <C>show</C>. <C>frame.draw</C> and{" "}
        <C>gpu.pipeline</C> are specified but not built yet: they come with the shaders below.
      </Note>
      <H2>Coming next: shaders in Kavach</H2>
      <P>
        Triangles will be drawn by two kinds of <C>gpu fn</C>: a <C>gpu vertex fn</C> works out where each corner goes
        on the screen, and a <C>gpu fragment fn</C> gives the color of each pixel inside. This is the spec&apos;s
        example; it doesn&apos;t compile yet:
      </P>
      <CodeBlock code={specBlockWith("15-gpu.md", "15.3", "gpu vertex fn")} file="from spec 15.3, not built yet" />
      <P>
        A list a GPU <C>map</C> made, and that is still in GPU memory, will be drawn from there with nothing copied: a
        particle swarm moved by a <C>map</C> each frame is drawn where it is. See{" "}
        <Link href="/docs/speed/gpu/">GPU</Link> for <C>gpu fn</C> and <C>map</C> today.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.24 · std.gpu" />
        <SrcRef file="docs/spec/15-gpu.md" label="spec 15.3 · drawing" />
        <SrcRef file="tests/conformance/14-standard-library/screens-draw-or-say-why.kav" label="screens, tested" />
        <SrcRef file="stdlib/gpu/gpu.kav" label="gpu.kav" />
        <SrcRef file="docs/decisions/0106-drawing-on-the-gpu.md" label="decision 0106" />
      </div>
    </>
  ),
};
