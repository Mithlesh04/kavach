import type { LaneState, SceneFrame } from "./types";

// Who runs when: one row per task, thread, core or call, one column per
// step, filled up to the step shown; later steps are dotted outlines. Each
// state has its own pattern as well as its color, and the row ends with
// this step's state in words, and where it runs ("thread 1 on core 0")
// when the scene says. Long scenes scroll in their own box. It generalises
// components/task-turns.tsx.

const LOOK: Record<LaneState, string> = {
  run: "bg-accent",
  ready: "border-[1.5px] border-accent bg-transparent",
  wait: "border-[1.5px] border-dashed border-soft bg-transparent",
  sleep: "bg-[repeating-linear-gradient(135deg,var(--line)_0_3px,transparent_3px_6px)] border border-line",
  done: "bg-ok-bg border border-ok",
};

const WORD: Record<LaneState, string> = { run: "runs", ready: "ready", wait: "waits", sleep: "sleeps", done: "done" };

export function TimelinePanel({ lanes, frames, step, title }: { lanes: string[]; frames: SceneFrame[]; step: number; title: string }) {
  const now = frames[step];
  const used = new Set(frames.flatMap((f) => Object.values(f.lanes ?? {})));
  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-lg border border-line bg-bg p-3" role="group" aria-label={`Timeline: ${title}`}>
      <span className="flex flex-wrap items-baseline gap-x-2 text-[12px] font-bold text-ink">
        Timeline <span className="font-normal text-soft">· {title}, step by step</span>
        {now.time && <span className="font-mono font-normal text-muted">· at {now.time}</span>}
      </span>
      <div className="overflow-x-auto">
        <div className="grid w-max items-center gap-x-[3px] gap-y-1" style={{ gridTemplateColumns: `auto repeat(${frames.length}, 16px) auto` }}>
          {lanes.map((lane) => {
            const state = now.lanes?.[lane];
            return (
              <div key={lane} className="contents">
                <span className="pr-2 font-mono text-[11px] text-muted">{lane}</span>
                {frames.map((f, i) => {
                  const s = i <= step ? f.lanes?.[lane] : undefined;
                  return (
                    <span
                      key={i}
                      title={s ? `${lane} ${WORD[s]} at step ${i + 1}` : undefined}
                      className={`h-4 rounded-[3px] ${s ? LOOK[s] : i > step ? "border border-dotted border-line" : ""} ${i === step ? "outline outline-1 outline-offset-1 outline-[var(--soft)]" : ""}`}
                    />
                  );
                })}
                <span className={`pl-2 text-[11px] ${state === "run" ? "font-bold text-ink" : "text-soft"}`}>
                  {state ? WORD[state] : "—"}
                  {now.where?.[lane] && <span className="font-normal text-soft"> · {now.where[lane]}</span>}
                </span>
              </div>
            );
          })}
        </div>
      </div>
      <span className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-soft">
        {(Object.keys(LOOK) as LaneState[])
          .filter((s) => used.has(s))
          .map((s) => (
            <span key={s} className="inline-flex items-center gap-1.5">
              <span className={`inline-block h-3 w-4 rounded-[3px] ${LOOK[s]}`} /> {WORD[s]}
            </span>
          ))}
      </span>
    </div>
  );
}
