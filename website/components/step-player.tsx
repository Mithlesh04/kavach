"use client";

import { useEffect, useState } from "react";

/**
 * Steps through the frames of an explainer picture: Back, Play and Next,
 * a slider to jump to any step, and the frame's caption. Play stops at
 * the last frame. Nothing moves by itself, so a reader who prefers less
 * motion is never surprised.
 */
export function useSteps(count: number, ms = 900) {
  const [frame, setFrame] = useState(0);
  const [wantsPlay, setPlaying] = useState(false);
  const playing = wantsPlay && frame < count - 1;
  useEffect(() => {
    if (!playing) return;
    const t = setTimeout(() => setFrame((f) => f + 1), ms);
    return () => clearTimeout(t);
  }, [playing, frame, ms]);
  return { frame, setFrame, playing, setPlaying, count };
}

/**
 * The controls. They are real buttons; with focus anywhere in them, the
 * arrow keys step, Home goes to the first step and End to the last. The
 * caption is read out by screen readers as it changes.
 */
export function StepControls({
  steps,
  caption,
}: {
  steps: ReturnType<typeof useSteps>;
  caption: React.ReactNode;
}) {
  const { frame, setFrame, playing, setPlaying, count } = steps;
  const go = (to: number) => {
    setPlaying(false);
    setFrame(Math.min(count - 1, Math.max(0, to)));
  };
  const keys = (e: React.KeyboardEvent) => {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
    const to = { ArrowLeft: frame - 1, ArrowRight: frame + 1, Home: 0, End: count - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    go(to);
  };
  const button =
    "cursor-pointer rounded-md border border-line bg-bg px-3 py-1 text-[13px] font-semibold text-ink disabled:cursor-default disabled:opacity-40";
  return (
    <div className="flex flex-col gap-2" onKeyDown={keys}>
      <div className="flex flex-wrap items-center gap-2">
        <button className={button} disabled={frame === 0} onClick={() => go(frame - 1)}>
          ◀ Back
        </button>
        <button
          className={button}
          onClick={() => {
            if (frame >= count - 1) {
              setFrame(0);
              setPlaying(true);
            } else {
              setPlaying(!playing);
            }
          }}
        >
          {playing ? "Pause" : frame >= count - 1 ? "Play again" : "Play"}
        </button>
        <button className={button} disabled={frame >= count - 1} onClick={() => go(frame + 1)}>
          Next ▶
        </button>
        {count > 2 && (
          <input
            type="range"
            min={0}
            max={count - 1}
            value={frame}
            onChange={(e) => go(Number(e.target.value))}
            aria-label="Go to step"
            aria-valuetext={`step ${frame + 1} of ${count}`}
            className="w-28 min-w-0 cursor-pointer accent-[var(--accent)] sm:w-40"
          />
        )}
        <span className="text-[12.5px] text-soft">
          step {frame + 1} of {count}
        </span>
      </div>
      <p className="m-0 min-h-[3em] text-[14.5px] leading-relaxed text-muted" aria-live="polite">
        {caption}
      </p>
    </div>
  );
}
