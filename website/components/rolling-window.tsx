"use client";

import { useState } from "react";

// Hours since the start of day 1 at which earlier calls ran; the limit
// is `count <= 3 per day`, a rolling 24 hours (spec 8.3).
const CALLS = [10, 18, 22];
const LIMIT = 3;
const SPAN = 48;

const clock = (h: number) => `day ${Math.floor(h / 24) + 1}, ${String(Math.floor(h % 24)).padStart(2, "0")}:00`;

/**
 * A rolling window you can move: the calls of the last 24 hours before
 * "now" count toward the limit, and midnight changes nothing. Worked out
 * from the calls above as the slider moves, as the runtime does.
 */
export function RollingWindow() {
  const [now, setNow] = useState(29);
  const from = now - 24;
  const inside = CALLS.filter((h) => h > from && h <= now);
  const allowed = inside.length < LIMIT;
  const pct = (h: number) => `${(Math.min(Math.max(h, 0), SPAN) / SPAN) * 100}%`;
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="A rolling 24-hour window on a timeline">
      <p className="m-0 text-sm font-extrabold">
        <code>count &lt;= 3 per day</code>, with three calls on day 1
      </p>
      <div className="relative mx-2 mt-6 mb-7 h-10">
        <div className="absolute inset-x-0 top-1/2 h-px bg-line" />
        <div
          className="absolute top-0 h-full rounded-md border border-accent bg-accent-soft transition-all duration-200"
          style={{ left: pct(from), width: `calc(${pct(now)} - ${pct(from)})` }}
          aria-hidden="true"
        />
        {[
          [0, "day 1 starts", "items-start"],
          [24, "midnight", "-translate-x-1/2 items-center"],
          [48, "day 2 ends", "-translate-x-full items-end"],
        ].map(([h, text, place]) => (
          <div key={h} className={`absolute -top-6 flex flex-col ${place}`} style={{ left: pct(Number(h)) }}>
            <span className="whitespace-nowrap text-[11px] text-soft">{text}</span>
            <span className="h-[4.5rem] border-l border-dashed border-line-2" />
          </div>
        ))}
        {CALLS.map((h) => {
          const counted = h > from && h <= now;
          return (
            <span
              key={h}
              title={clock(h)}
              className={`absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 ${counted ? "border-accent bg-accent" : "border-line bg-bg"}`}
              style={{ left: pct(h) }}
            />
          );
        })}
        <span className="absolute top-1/2 h-6 w-0.5 -translate-x-1/2 -translate-y-1/2 bg-ink" style={{ left: pct(now) }} />
        <span className="absolute -bottom-6 -translate-x-1/2 text-[11px] font-bold text-ink" style={{ left: pct(now) }}>
          now
        </span>
      </div>
      <label className="flex flex-col gap-1.5 text-[13px] text-muted">
        Move &quot;now&quot;: {clock(now)}
        <input type="range" min={0} max={SPAN} value={now} onChange={(e) => setNow(Number(e.target.value))} className="w-full accent-accent" />
      </label>
      <p className="m-0 text-[14.5px] leading-relaxed text-muted" aria-live="polite">
        The window is {clock(Math.max(from, 0))} to {clock(now)}. It holds{" "}
        <strong className="text-ink">{inside.length}</strong> of the calls, so a call now is{" "}
        <strong className={allowed ? "text-ok" : "text-bad"}>{allowed ? "allowed" : "refused: it would be the fourth in 24 hours"}</strong>.
        {now >= 24 && inside.some((h) => h < 24) && " Midnight has passed, yet day 1's calls still count: nothing resets at midnight."}
      </p>
    </div>
  );
}
