import type { PipelineView, RingView } from "./types";

// Two views of things that flow: a pipeline strip (stages in a row, items
// moving one stage per step: the compiler's stages, a CPU's, a GPU's draw
// pipeline, map → filter → sum) and a ring buffer (a fixed row of cells
// with a place to take from and a place to put into: a channel's buffer,
// an output buffer). Both scroll inside their own box when narrow.

const chip = "rounded-full bg-accent px-2 py-0.5 font-mono text-[11px] font-bold text-panel";

/** Stages left to right; each shows the items in it now. Items past the last stage are out. */
export function PipelineStrip({ view }: { view: PipelineView }) {
  const out = view.items.filter((it) => it.stage >= view.stages.length);
  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-lg border border-line bg-bg p-3" role="group" aria-label={view.title}>
      <span className="text-[12px] font-bold text-ink">{view.title}</span>
      <div className="overflow-x-auto pb-1">
        <ol className="m-0 flex w-max list-none items-stretch gap-1 p-0">
          {view.stages.map((stage, i) => {
            const here = view.items.filter((it) => it.stage === i);
            return (
              <li key={stage} className="flex items-stretch gap-1">
                <div className={`flex w-[104px] flex-col gap-1.5 rounded-md border p-2 ${here.length ? "border-accent bg-accent-soft" : "border-line bg-panel"}`}>
                  <span className="text-[11px] font-bold leading-tight text-ink">
                    <span className="text-soft">{i + 1}.</span> {stage}
                  </span>
                  <span className="flex min-h-[22px] flex-wrap gap-1">
                    {here.map((it) => (
                      <span key={it.label} className={`mem-in ${chip}`}>
                        {it.label}
                      </span>
                    ))}
                  </span>
                  {view.notes?.[i] && <span className="text-[10.5px] font-semibold text-bad">{view.notes[i]}</span>}
                </div>
                <span className="self-center text-soft" aria-hidden="true">
                  →
                </span>
              </li>
            );
          })}
          <li className="flex w-[84px] flex-col gap-1.5 rounded-md border border-dashed border-line p-2">
            <span className="text-[11px] font-bold text-soft">out</span>
            <span className="flex flex-wrap gap-1">
              {out.map((it) => (
                <span key={it.label} className="rounded-full border border-ok px-2 py-0.5 font-mono text-[11px] text-ink">
                  {it.label}
                </span>
              ))}
            </span>
          </li>
        </ol>
      </div>
    </div>
  );
}

/** A ring of cells in a row: "take" marks the head, "put" the tail; the last cell wraps to the first. */
export function RingBuffer({ view }: { view: RingView }) {
  const n = view.cells.length;
  const used = view.cells.filter((c) => c !== null).length;
  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-lg border border-line bg-bg p-3" role="group" aria-label={view.title}>
      <span className="flex flex-wrap items-baseline gap-x-2 text-[12px] font-bold text-ink">
        {view.title}
        <span className="font-normal text-soft">
          {used} of {n} cells used{view.closed ? " · closed: no more puts" : ""}
        </span>
      </span>
      <div className="overflow-x-auto pb-1">
        <div className="flex w-max items-start gap-1">
          {view.cells.map((c, i) => (
            <div key={i} className="flex w-14 flex-col items-center gap-0.5">
              <span
                className={`flex h-10 w-full items-center justify-center rounded-[4px] border font-mono text-[12px] font-semibold ${c === null ? "border-dashed border-line text-soft" : "border-accent bg-accent-soft text-ink"}`}
              >
                {c ?? "empty"}
              </span>
              <span className="font-mono text-[9.5px] text-soft">{i}</span>
              <span className="flex flex-col items-center text-[10.5px] font-bold leading-tight">
                {i === view.head && <span className="text-ch-c">▲ take</span>}
                {i === view.tail && <span className="text-accent-text">▲ put</span>}
              </span>
            </div>
          ))}
          <span className="self-start pt-2.5 text-[11px] text-soft" aria-hidden="true">
            ↩ to 0
          </span>
        </div>
      </div>
      {view.waiting?.length ? (
        <ul className="m-0 flex list-none flex-wrap gap-x-4 gap-y-1 p-0 text-[11.5px] text-muted">
          {view.waiting.map((w) => (
            <li key={w.who}>
              <span className="font-mono font-bold text-ink">{w.who}</span> waits for {w.for === "room" ? "room" : "an item"}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
