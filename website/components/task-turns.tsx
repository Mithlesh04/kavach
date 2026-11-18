"use client";

import { StepControls, useSteps } from "./step-player";

type Lane = "main" | "slow" | "fast";
type State = "runs" | "waits" | "ready" | "done" | "";

type Frame = { at: string; lanes: Record<Lane, State>; printed: number; caption: string };

// The turns of tasks/sub-turns, as spec 9.7 defines them, for the sleeps the
// sample gives slow and fast. `printed` counts how many of the sample's
// checked output lines exist at that moment.
const frames = (slow: number, fast: number): Frame[] => [
  { at: "0 ms", lanes: { main: "runs", slow: "", fast: "" }, printed: 0, caption: "main runs. spawn starts a task and makes it ready; the task that spawned it keeps running." },
  { at: "0 ms", lanes: { main: "runs", slow: "ready", fast: "ready" }, printed: 1, caption: "Both tasks are ready, but main still has the turn, so it prints first." },
  { at: "0 ms", lanes: { main: "waits", slow: "waits", fast: "waits" }, printed: 1, caption: `main waits for slow. A waiting task gives up its turn: slow runs and starts sleeping ${slow} ms, then fast starts sleeping ${fast} ms. The sleeps overlap.` },
  { at: `${fast} ms`, lanes: { main: "waits", slow: "waits", fast: "done" }, printed: 2, caption: "fast's sleep ends first, so fast is ready first: it prints and ends." },
  { at: `${slow} ms`, lanes: { main: "waits", slow: "done", fast: "done" }, printed: 3, caption: "slow wakes, prints and ends, which is what main waited for." },
  { at: `${slow} ms`, lanes: { main: "runs", slow: "done", fast: "done" }, printed: 4, caption: `main runs again. Both results are there: about ${slow} ms in all, not ${slow + fast}, and the same order on every run.` },
];

const style: Record<State, string> = {
  runs: "border-accent bg-accent-soft text-ink",
  waits: "border-line text-soft border-dashed",
  ready: "border-line text-muted",
  done: "border-ok bg-ok-bg text-ink",
  "": "border-transparent text-transparent",
};

/** Tasks taking turns (spec 9.7), step by step, beside what the program printed. */
export function TaskTurns({ output, slow, fast }: { output: string[]; slow: number; fast: number }) {
  const all = frames(slow, fast);
  const steps = useSteps(all.length, 1900);
  const f = all[steps.frame];
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="Tasks taking turns, step by step">
      <div className="grid gap-4 sm:grid-cols-[1fr_1fr]">
        <div className="flex flex-col gap-2">
          <span className="text-[12px] font-bold text-soft">at {f.at}</span>
          {(["main", "slow", "fast"] as Lane[]).map((lane) => (
            <div key={lane} className="flex items-center gap-3">
              <span className="w-10 font-mono text-[13px] text-ink">{lane}</span>
              <span className={`flex-1 rounded-md border px-3 py-1.5 text-[13px] transition-colors duration-300 ${style[f.lanes[lane]]}`}>
                {f.lanes[lane] || "not started"}
              </span>
            </div>
          ))}
        </div>
        <div className="flex flex-col gap-1 rounded-md border border-line bg-bg p-3 font-mono text-[12.5px]">
          <span className="mb-1 font-sans text-[11px] font-bold uppercase tracking-wide text-soft">printed</span>
          {output.slice(0, f.printed).map((line, i) => (
            <span key={i} className={i === f.printed - 1 ? "text-ink" : "text-muted"}>{line}</span>
          ))}
        </div>
      </div>
      <StepControls steps={steps} caption={f.caption} />
    </div>
  );
}
