"use client";

import { StepControls, useSteps } from "./step-player";

const ROUNDS = 16;
const CORES = 4;
const CHUNK = ROUNDS / CORES;

type Frame = { done: (i: number) => boolean; split: boolean; replay?: boolean; caption: string };

// Decision 0031: chunks on all cores, results exactly the plain loop's.
const FRAMES: Frame[] = [
  { done: () => false, split: false, caption: "A loop of 16 rounds, each writing out[i] from xs[i]. Written plainly, one core would run round 0, then 1, then 2…" },
  { done: () => false, split: true, caption: "Values never alias in Kavach, so the compiler can prove no round touches another's memory. It cuts the loop into one chunk per core." },
  { done: (i) => i % CHUNK < 2, split: true, caption: "Every core runs its own chunk at the same time." },
  { done: () => true, split: true, caption: "Each result lands in its own place: the list is exactly what the plain loop would have made, with 4 cores working at once." },
  { done: (i) => i < 6, split: false, replay: true, caption: "If any chunk had panicked, the results would be thrown away and the loop run again on one core from round 0, so it panics at the same round, with the same message, as the plain loop." },
];

/**
 * A loop on all cores (decision 0031): split into chunks, run at once,
 * results in place, and a serial replay when a chunk panics. 16 rounds
 * and 4 cores keep the picture small; the runtime uses every core, and
 * only for loops big enough to be worth it.
 */
export function CoresSplit() {
  const steps = useSteps(FRAMES.length, 1900);
  const f = FRAMES[steps.frame];
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="A loop split over all cores">
      <code className="text-[13px]">{"for i in 0..16 { out[i] = math.sqrt(xs[i]) }"}</code>
      <div className={`grid gap-2 ${f.split ? "grid-cols-2 sm:grid-cols-4" : "grid-cols-1"}`}>
        {(f.split ? Array.from({ length: CORES }, (_, c) => c) : [0]).map((c) => {
          const rounds = f.split ? Array.from({ length: CHUNK }, (_, k) => c * CHUNK + k) : Array.from({ length: ROUNDS }, (_, k) => k);
          return (
            <div key={c} className="flex flex-col gap-1.5 rounded-lg border border-line bg-bg p-2">
              <span className="text-[11px] font-bold text-soft">{f.split ? `core ${c + 1}` : f.replay ? "one core, from round 0" : "one core"}</span>
              <div className="flex flex-wrap gap-1">
                {rounds.map((i) => (
                  <span
                    key={i}
                    className={`flex h-7 w-7 items-center justify-center rounded border font-mono text-[11px] transition-colors duration-300 ${
                      f.done(i) ? "border-ok bg-ok-bg text-ink" : "border-line text-soft"
                    }`}
                  >
                    {i}
                  </span>
                ))}
              </div>
            </div>
          );
        })}
      </div>
      <StepControls steps={steps} caption={f.caption} />
      <p className="m-0 text-[12.5px] text-soft">
        Loops too small to gain stay on one core, and so do sums of floats, whose result depends on the order of adding.
      </p>
    </div>
  );
}
