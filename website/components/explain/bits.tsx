import type { BitsView } from "./types";

// Two small views for numbers: a value as its bits (whole numbers in two's
// complement, floats as sign, exponent and fraction), with the CPU's flags;
// and a latency ladder, on a log scale, for how long each kind of memory
// takes to answer. The ladder never holds a figure of its own: its rows
// come from a data file that cites where each figure is from.

const TONE = {
  sign: "border-t-[3px] border-t-[var(--ch-p)]",
  exp: "border-t-[3px] border-t-[var(--ch-c)]",
  frac: "border-t-[3px] border-t-[var(--ch-go)]",
  plain: "",
};

/** One value's bits, a byte to a group so that 64 bits wrap on a phone; a group's name says what its bits are. */
export function BitsRow({ view }: { view: BitsView }) {
  const tone = (i: number) => view.groups?.find((g) => i >= g.from && i <= g.to)?.tone ?? "plain";
  const bytes = Array.from({ length: Math.ceil(view.bits.length / 8) }, (_, b) => view.bits.slice(b * 8, b * 8 + 8));
  return (
    <div className="flex min-w-0 flex-col gap-2 rounded-lg border border-line bg-bg p-3" role="group" aria-label={view.title}>
      <span className="flex flex-wrap items-baseline gap-x-2 text-[12px] font-bold text-ink">
        {view.title}
        {view.value && <span className="font-mono font-normal text-muted">= {view.value}</span>}
      </span>
      <div className="flex flex-wrap gap-x-2 gap-y-1.5" aria-label={`bits ${view.bits}`}>
        {bytes.map((byte, b) => (
          <span key={b} className="flex gap-px">
            {[...byte].map((bit, j) => {
              const i = b * 8 + j;
              return (
                <span
                  key={j}
                  className={`flex h-6 w-[15px] items-center justify-center rounded-[2px] border border-line font-mono text-[11px] ${TONE[tone(i)]} ${bit === "1" ? "bg-chip font-bold text-ink" : "bg-panel text-soft"}`}
                >
                  {bit}
                </span>
              );
            })}
          </span>
        ))}
      </div>
      {view.groups && (
        <span className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-soft">
          {view.groups.map((g) => (
            <span key={g.label} className="inline-flex items-center gap-1.5">
              <span className={`inline-block h-3 w-4 rounded-[2px] border border-line ${TONE[g.tone]}`} /> {g.label}: bits {g.from}–{g.to}
            </span>
          ))}
        </span>
      )}
      {view.flags && (
        <span className="flex flex-wrap gap-1.5">
          {view.flags.map((f) => (
            <span key={f.name} className={`rounded-md border px-2 py-0.5 font-mono text-[11px] ${f.on ? "border-bad bg-bad-bg font-bold text-bad" : "border-line text-soft"}`}>
              {f.name}: {f.on ? "1, set" : "0"}
            </span>
          ))}
        </span>
      )}
    </div>
  );
}

/** One rung: a kind of memory, how long it takes to answer in nanoseconds, and where the figure is from. */
export type Rung = { name: string; ns: number; source: string };

const unit = (ns: number) =>
  ns < 1000 ? `${ns} ns` : ns < 1e6 ? `${+(ns / 1e3).toPrecision(3)} µs` : `${+(ns / 1e6).toPrecision(3)} ms`;

/** How long each kind of memory takes to answer, on a log scale; `lit` marks the rung a step is about. */
export function LatencyLadder({ rungs, lit, title }: { rungs: Rung[]; lit?: string; title: string }) {
  const lo = Math.log10(Math.min(...rungs.map((r) => r.ns)) / 4);
  const hi = Math.log10(Math.max(...rungs.map((r) => r.ns)));
  const width = (ns: number) => `${Math.max(2, ((Math.log10(ns) - lo) / (hi - lo)) * 100)}%`;
  const sources = [...new Set(rungs.map((r) => r.source))];
  return (
    <figure className="m-0 flex min-w-0 flex-col gap-2 rounded-lg border border-line bg-bg p-3">
      <figcaption className="text-[12px] font-bold text-ink">
        {title} <span className="font-normal text-soft">· log scale: each equal step of width is the same number of times slower</span>
      </figcaption>
      <dl className="m-0 grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-1.5">
        {rungs.map((r) => (
          <div key={r.name} className="contents">
            <dt className={`text-[12px] ${r.name === lit ? "font-bold text-ink" : "text-muted"}`}>{r.name}</dt>
            <dd className="m-0 flex min-w-0 items-center gap-2">
              <span className={`h-3 rounded-[3px] ${r.name === lit ? "bg-accent" : "bg-line"}`} style={{ width: width(r.ns) }} />
              <span className="shrink-0 font-mono text-[11px] text-soft">{unit(r.ns)}</span>
            </dd>
          </div>
        ))}
      </dl>
      <span className="text-[11px] text-soft">Typical figures for today&apos;s hardware, not Kavach measurements. From: {sources.join("; ")}.</span>
    </figure>
  );
}
