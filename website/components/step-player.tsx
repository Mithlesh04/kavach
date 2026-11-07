"use client";

import { useEffect, useState } from "react";

/**
 * Steps through the frames of an explainer picture: Back, Play and Next,
 * and the frame's caption. Play stops at the last frame. Nothing moves
 * by itself, so a reader who prefers less motion is never surprised.
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

export function StepControls({
  steps,
  caption,
}: {
  steps: ReturnType<typeof useSteps>;
  caption: React.ReactNode;
}) {
  const { frame, setFrame, playing, setPlaying, count } = steps;
  const button =
    "cursor-pointer rounded-md border border-line bg-bg px-3 py-1 text-[13px] font-semibold text-ink disabled:cursor-default disabled:opacity-40";
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <button className={button} disabled={frame === 0} onClick={() => { setPlaying(false); setFrame(frame - 1); }}>
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
        <button className={button} disabled={frame >= count - 1} onClick={() => { setPlaying(false); setFrame(frame + 1); }}>
          Next ▶
        </button>
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
