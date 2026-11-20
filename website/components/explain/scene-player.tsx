"use client";

import { StepControls, useSteps } from "../step-player";
import { BitsRow } from "./bits";
import { Board } from "./board";
import { CodeLines, Printed } from "./code-track";
import { CallStackPanel, HeapPanel, RegistersPanel } from "./panels";
import { PipelineStrip, RingBuffer } from "./strips";
import { TimelinePanel } from "./timeline";
import type { Scene } from "./types";

// A scene, step by step: the code with the running line marked, the
// machine with its lit parts and moving packets, then the panels the scene
// asks for (registers, call stack, heap, timeline) and the views a frame
// carries (pipeline, ring buffer, bits). explainer.tsx checks the scene
// against its sample and writes every step out as text under it.

export function ScenePlayer({ scene, html }: { scene: Scene; html: string[] }) {
  const steps = useSteps(scene.frames.length, 2400);
  const f = scene.frames[steps.frame];
  const prev = scene.frames[steps.frame - 1];
  const panels = scene.panels ?? [];
  const printed = scene.frames.some((x) => x.output !== undefined);
  return (
    <div className="flex min-w-0 flex-col gap-4 rounded-xl border border-line bg-panel px-4 py-4 sm:px-5" role="group" aria-label={`${scene.title}, step by step`}>
      <div className="grid grid-cols-[minmax(0,1fr)] gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
        <div className="flex min-w-0 flex-col gap-2">
          <CodeLines html={html} line={f.line} lastUse={scene.lastUse} />
          {printed && <Printed text={f.output ?? ""} label="Printed so far (checked against a real run):" />}
        </div>
        <div className="flex min-w-0 flex-col items-center justify-start">
          <Board scene={scene} frame={f} step={steps.frame} />
        </div>
      </div>
      {panels.some((p) => p !== "timeline") && (
        <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,230px),1fr))] items-start gap-3">
          {panels.includes("regs") && <RegistersPanel regs={f.regs ?? []} before={prev?.regs} />}
          {panels.includes("stack") && <CallStackPanel stack={f.stack ?? []} before={prev?.stack} />}
          {panels.includes("heap") && <HeapPanel heap={f.heap ?? []} empty={scene.heapEmpty ?? "Nothing on the heap."} />}
          {panels.includes("vram") && <HeapPanel heap={f.vram ?? []} empty="Nothing of this program's yet." title="GPU memory" hint="VRAM, on the graphics card" />}
        </div>
      )}
      {f.pipeline && <PipelineStrip view={f.pipeline} />}
      {f.ring && <RingBuffer view={f.ring} />}
      {f.bits?.map((b) => <BitsRow key={b.title} view={b} />)}
      {panels.includes("timeline") && scene.lanes && (
        <TimelinePanel lanes={scene.lanes} frames={scene.frames} step={steps.frame} title={scene.lanesAre ?? "who runs"} />
      )}
      <StepControls steps={steps} caption={f.caption} />
      {scene.note && <p className="m-0 text-[12px] leading-relaxed text-soft">{scene.note}</p>}
    </div>
  );
}
