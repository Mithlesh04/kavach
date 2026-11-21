"use client";

import { StepControls, useSteps } from "./step-player";

// A loop `on cores` (spec 4.9, decision 0031), as an LLVM-built program
// runs it (runtime/native/src/rt_par.rs, fan_out and run_shares): the
// rounds are cut into many small ranges, and the thread running the loop
// and the runtime's kept threads each take the next free range from a
// shared counter until none are left. Which core takes which range
// changes from run to run; the picture shows one run. A loop under
// 65,536 rounds, reading more than 6 values from outside it, or on one
// core runs as the plain loop, and so does every loop in a program
// Cranelift or the interpreter runs.

const RANGES = 16;
const CORES = 4;

/** Who took each range in this example run: [range, core]. */
type Taken = [number, number][];

type Frame = { done: Taken; taking: Taken; split: boolean; replay?: boolean; caption: string };

const first: Taken = [[0, 1], [1, 2], [2, 3], [3, 4]];
const second: Taken = [[4, 3], [5, 1], [6, 4], [7, 2]];
const rest: Taken = [[8, 2], [9, 3], [10, 1], [11, 4], [12, 3], [13, 2], [14, 4], [15, 1]];

const FRAMES: Frame[] = [
  { done: [], taking: [], split: false, caption: "A loop of many rounds, each writing out[i] from xs[i]. Written plainly, one core runs round 0, then 1, then 2…" },
  { done: [], taking: first, split: true, caption: "With on cores, the rounds are cut into many small ranges, and each core takes the next free range until none are left. Here 4 cores start on ranges 0 to 3: the thread running the loop, and threads the runtime keeps for loops." },
  { done: first, taking: second, split: true, caption: "A core that finishes a range takes the next free one at once, so no core waits for a slow one. Which core takes which range changes from run to run: this is one run." },
  { done: [...first, ...second, ...rest], taking: [], split: true, caption: "No range is left, so the loop is over. Each round wrote only its own slot, so the list is exactly what the plain loop would have made." },
  { done: [], taking: [], split: false, replay: true, caption: "If any range had panicked, the results would be thrown away and the loop run again on one core from round 0, so it panics at the same round, with the same message, as the plain loop." },
];

const coreOf = (t: Taken, r: number) => t.find(([x]) => x === r)?.[1];

/** A loop on all cores: small ranges taken in turn, results in place, a one-core replay when a range panics. */
export function CoresSplit() {
  const steps = useSteps(FRAMES.length, 1900);
  const f = FRAMES[steps.frame];
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="A loop split over all cores">
      <code className="text-[13px]">{"on cores for i in 0..n { out[i] = math.sqrt(xs[i]) }"}</code>
      <span className="text-[12px] font-bold text-soft">
        {f.split ? `the rounds in ${RANGES} small ranges, ${CORES} cores taking them` : f.replay ? "one core, from round 0; each box is a block of rounds" : "one core, in order; each box is a block of rounds"}
      </span>
      <div className="grid grid-cols-4 gap-1.5 sm:grid-cols-8">
        {Array.from({ length: RANGES }, (_, r) => {
          const done = coreOf(f.done, r);
          const now = coreOf(f.taking, r);
          const replayed = f.replay && r < 6;
          return (
            <div
              key={r}
              className={`flex h-12 flex-col items-center justify-center rounded border font-mono text-[11px] transition-colors duration-300 ${
                now ? "border-accent bg-accent-soft text-ink" : done || replayed ? "border-ok bg-ok-bg text-ink" : "border-line text-soft"
              }`}
            >
              <span>{f.split ? `range ${r}` : r}</span>
              <span className="text-[10px] text-soft">{now ? `core ${now} …` : done ? `core ${done}` : ""}</span>
            </div>
          );
        })}
      </div>
      <StepControls steps={steps} caption={f.caption} />
      <p className="m-0 text-[12.5px] text-soft">
        Only a program built with kavach build (LLVM) spreads a loop. Loops of fewer than 65,536 rounds, loops that read more
        than 6 values from outside them, and programs on one core run the plain loop. A sum of floats is refused, as
        its result depends on the order of adding.
      </p>
    </div>
  );
}
