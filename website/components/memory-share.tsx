"use client";

import { StepControls, useSteps } from "./step-player";

type Box = { items: number[]; holders: string[]; changed?: boolean };
type Frame = { code: string; boxes: Box[]; freed?: string; caption: string };

// Spec 6.1, "How it stays fast" and "Freed immediately", one line at a time.
const FRAMES: Frame[] = [
  {
    code: "let a = [1, 2, 3]",
    boxes: [{ items: [1, 2, 3], holders: ["a"] }],
    caption: "The list is made once in memory. One variable holds it, so its count is 1.",
  },
  {
    code: "let b = a",
    boxes: [{ items: [1, 2, 3], holders: ["a", "b"] }],
    caption: "Nothing is copied: a and b share the list inside, and its count becomes 2. To your code they are still two separate values.",
  },
  {
    code: "b.push(4)",
    boxes: [
      { items: [1, 2, 3], holders: ["a"] },
      { items: [1, 2, 3, 4], holders: ["b"], changed: true },
    ],
    caption: "b changes while a still holds the list, so now, and only now, the list is copied. a keeps [1, 2, 3]: no one else can change your value.",
  },
  {
    code: "print(a)   // a's last use",
    boxes: [{ items: [1, 2, 3, 4], holders: ["b"] }],
    freed: "[1, 2, 3]",
    caption: "After its last use, a's list is freed at once: no garbage collector, no pause later, and memory comes back at a point you can predict.",
  },
  {
    code: "b.push(5)",
    boxes: [{ items: [1, 2, 3, 4, 5], holders: ["b"], changed: true }],
    caption: "Nobody else holds b's list, so it is changed in place, with no copy. Most count updates are removed while compiling, so this costs about what hand-managed memory does.",
  },
];

/** Values, not references: what happens in memory while value code runs (spec 6.1). */
export function MemoryShare() {
  const steps = useSteps(FRAMES.length, 1900);
  const f = FRAMES[steps.frame];
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-5 py-5" role="group" aria-label="What memory does while values are shared, changed and freed">
      <div className="flex flex-col gap-1 font-mono text-[13px]">
        {FRAMES.map((fr, i) => (
          <span key={fr.code} className={i === steps.frame ? "font-bold text-ink" : i < steps.frame ? "text-soft" : "text-soft opacity-40"}>
            {i === steps.frame ? "▶ " : "  "}
            {fr.code}
          </span>
        ))}
      </div>
      <div className="flex min-h-[92px] flex-wrap items-start gap-4 border-t border-dashed border-line-2 pt-4">
        {f.boxes.map((b) => (
          <div key={b.holders.join()} className="flex flex-col gap-1.5">
            <span className="text-[12px] text-soft">
              held by <strong className="text-ink">{b.holders.join(" and ")}</strong> · count {b.holders.length}
            </span>
            <div className={`flex gap-1 rounded-lg border-[1.5px] p-1.5 transition-colors duration-300 ${b.changed ? "border-accent bg-accent-soft" : "border-line bg-bg"}`}>
              {b.items.map((x, i) => (
                <span key={i} className="flex h-8 w-8 items-center justify-center rounded border border-line font-mono text-[13px]">
                  {x}
                </span>
              ))}
            </div>
          </div>
        ))}
        {f.freed && (
          <div className="flex flex-col gap-1.5">
            <span className="text-[12px] text-ok">freed</span>
            <div className="rounded-lg border-[1.5px] border-dashed border-line px-3 py-2 font-mono text-[13px] text-soft line-through">{f.freed}</div>
          </div>
        )}
      </div>
      <StepControls steps={steps} caption={f.caption} />
    </div>
  );
}
