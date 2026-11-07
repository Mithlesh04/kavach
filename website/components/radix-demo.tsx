"use client";

import { StepControls, useSteps } from "./step-player";

const START = [170, 45, 75, 90, 802, 24, 2, 66];
const PLACES = [
  { div: 1, name: "ones" },
  { div: 10, name: "tens" },
  { div: 100, name: "hundreds" },
];

type Frame = { list: number[]; buckets?: number[][]; place?: number; caption: string };

/** Every frame of a least-significant-digit radix sort, worked out here, not typed in. */
function frames(): Frame[] {
  const out: Frame[] = [{ list: START, caption: "Eight numbers to sort. A radix sort never compares two numbers: it sorts by one digit at a time." }];
  let list = START;
  for (const [place, { div, name }] of PLACES.entries()) {
    const buckets: number[][] = Array.from({ length: 10 }, () => []);
    for (const n of list) buckets[Math.floor(n / div) % 10].push(n);
    out.push({
      list,
      buckets,
      place,
      caption: `Each number goes into the bucket of its ${name} digit, in the order it comes. Numbers in one bucket keep their order: that is what makes the next pass work.`,
    });
    list = buckets.flat();
    out.push({
      list,
      caption:
        div === 100
          ? "Read the buckets back in order: the list is sorted, after three passes over it and not a single comparison."
          : `Read the buckets back from 0 to 9. The list is now sorted by its ${name} digit; the next pass sorts by the digit to the left.`,
    });
  }
  return out;
}

const ALL = frames();

/** A number with the digit at `place` (0 for ones) in bold; missing digits show as faint zeros. */
function Digits({ n, place }: { n: number; place?: number }) {
  if (place === undefined) return <>{n}</>;
  const s = String(n);
  const padded = s.padStart(Math.max(s.length, place + 1), "0");
  const at = padded.length - 1 - place;
  const added = padded.length - s.length;
  return (
    <>
      {[...padded].map((ch, i) => (
        <span key={i} className={i === at ? "font-bold text-accent-text" : i < added ? "text-soft opacity-50" : ""}>
          {ch}
        </span>
      ))}
    </>
  );
}

/**
 * A radix sort, step by step, on decimal digits. Kavach's own uses 8-bit
 * digits, so 256 buckets and 8 passes for 64-bit numbers at most, fewer
 * when the high bits are the same in every number (runtime/native/src/radix.rs).
 */
export function RadixDemo() {
  const steps = useSteps(ALL.length, 1600);
  const f = ALL[steps.frame];
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="A radix sort, step by step">
      <div className="flex flex-wrap gap-1.5">
        {f.list.map((n, i) => (
          <span key={`${n}-${i}`} className="rounded-md border border-line bg-bg px-2.5 py-1.5 font-mono text-[13px]">
            <Digits n={n} place={f.place} />
          </span>
        ))}
      </div>
      {f.buckets && (
        <div className="grid grid-cols-5 gap-1.5 sm:grid-cols-10">
          {f.buckets.map((b, d) => (
            <div key={d} className={`flex min-h-24 flex-col items-center gap-1 rounded-md border px-1 py-1.5 ${b.length ? "border-accent bg-accent-soft" : "border-line"}`}>
              <span className="text-[11px] font-bold text-soft">{d}</span>
              {b.map((n, i) => (
                <span key={`${n}-${i}`} className="font-mono text-[12px] text-ink">{n}</span>
              ))}
            </div>
          ))}
        </div>
      )}
      <StepControls steps={steps} caption={f.caption} />
    </div>
  );
}
