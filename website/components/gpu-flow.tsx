"use client";

import { StepControls, useSteps } from "./step-player";

const ITEMS = [0.1, 0.4, 0.9, 0.2, 0.7, 0.5, 0.3, 0.8];

type Where = "cpu" | "going" | "gpu" | "coming";

// The GPU's frames: the list on the CPU, copied over, every item at once,
// copied back. The CPU's: one item per frame, on one core.
const gpuFrames: { where: Where; done: boolean; caption: string }[] = [
  { where: "cpu", done: false, caption: "The list is in the CPU's memory, as every list starts." },
  { where: "going", done: false, caption: "The runtime copies it to the GPU's memory. This trip costs time whatever the work is: it is why a short list, or light work, stays on the CPU." },
  { where: "gpu", done: true, caption: "One GPU thread per item: every item runs through the gpu fn at the same time. A GPU has thousands of these threads." },
  { where: "coming", done: true, caption: "The results come back into a new list. A list the GPU made stays in its memory for a while, so a second map of it skips the trip over." },
];

/**
 * How a GPU map differs from a CPU one: one item after another on a core,
 * against copy over, all items at once, copy back. Illustrative steps,
 * not measured times; the race chart on the page has the measurements.
 */
export function GpuFlow() {
  const cpu = useSteps(ITEMS.length + 1, 450);
  const gpu = useSteps(gpuFrames.length, 1300);
  const g = gpuFrames[gpu.frame];
  const cell = (on: boolean, active = false) =>
    `flex h-9 w-11 items-center justify-center rounded-md border font-mono text-[12px] transition-colors duration-300 ${
      active ? "border-accent bg-accent-soft text-ink" : on ? "border-ok text-ink" : "border-line text-soft"
    }`;

  return (
    <div className="flex flex-col gap-5 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="How a map runs on one CPU core and on the GPU">
      <section className="flex flex-col gap-3">
        <p className="m-0 text-sm font-extrabold">One CPU core: one item after another</p>
        <div className="flex flex-wrap gap-1.5">
          {ITEMS.map((x, i) => (
            <span key={i} className={cell(i < cpu.frame, i === cpu.frame && cpu.frame < ITEMS.length)}>
              {x}
            </span>
          ))}
        </div>
        <StepControls
          steps={cpu}
          caption={
            cpu.frame < ITEMS.length
              ? `Item ${cpu.frame + 1} of ${ITEMS.length}: the core works on it, then moves on. Each item waits its turn.`
              : `All ${ITEMS.length} done, in ${ITEMS.length} turns. A million items take a million turns.`
          }
        />
      </section>
      <section className="flex flex-col gap-3 border-t border-dashed border-line-2 pt-4">
        <p className="m-0 text-sm font-extrabold">The GPU: copy over, every item at once, copy back</p>
        <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2">
          <span className="text-[12px] font-bold text-soft">CPU memory</span>
          <div className="flex flex-wrap gap-1.5">
            {ITEMS.map((x, i) => (
              <span key={i} className={cell(g.where === "coming", g.where === "cpu")}>
                {g.where === "coming" ? "✓" : g.where === "cpu" ? x : ""}
              </span>
            ))}
          </div>
          <span className="text-[12px] font-bold text-soft">GPU threads</span>
          <div className="flex flex-wrap gap-1.5">
            {ITEMS.map((x, i) => (
              <span key={i} className={cell(g.where === "gpu", g.where === "going")}>
                {g.where === "going" ? x : g.where === "gpu" ? "✓" : ""}
              </span>
            ))}
          </div>
        </div>
        <StepControls steps={gpu} caption={g.caption} />
      </section>
      <p className="m-0 text-[12.5px] text-soft">A picture of the steps, not of the times: the charts below have the measured times.</p>
    </div>
  );
}
