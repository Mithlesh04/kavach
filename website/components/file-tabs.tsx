"use client";

import { useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { CopyButton } from "./copy-button";
import { Blades } from "./logo";

/**
 * A code window holding several files of one project: a tab for each, as in
 * an editor. Every file's lines are in the page already, colored at build
 * time, so a tab only chooses which to show.
 */
export function FileTabs({
  files,
  bodies,
  statuses,
  dots,
  terminal,
}: {
  files: { path: string; code: string }[];
  bodies: ReactNode[];
  statuses: ReactNode[];
  dots: ReactNode;
  terminal: ReactNode;
}) {
  const [active, setActive] = useState(0);
  const id = useId();
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);

  const onKey = (e: KeyboardEvent) => {
    const last = files.length - 1;
    const to = { ArrowRight: active === last ? 0 : active + 1, ArrowLeft: active === 0 ? last : active - 1, Home: 0, End: last }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    setActive(to);
    tabs.current[to]?.focus();
  };

  return (
    <>
      <div className="flex items-end gap-3 border-b border-code-line bg-[var(--term-bar)] pl-3.5 pr-2 max-sm:pl-2">
        {dots}
        {/* A phone can't fit every path: the tabs scroll, and the Copy button stays. -mb-px lets the open tab cover the bar's line. */}
        <div role="tablist" aria-label="Files of this program" onKeyDown={onKey} className="-mb-px flex min-w-0 flex-1 items-end gap-1 overflow-x-auto overflow-y-hidden [scrollbar-width:none]">
          {files.map((f, i) => (
            <button
              key={f.path}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              id={`${id}-tab-${i}`}
              role="tab"
              type="button"
              aria-selected={i === active}
              aria-controls={`${id}-file-${i}`}
              tabIndex={i === active ? 0 : -1}
              onClick={() => setActive(i)}
              className={`mt-1.5 flex shrink-0 cursor-pointer items-center gap-1.5 rounded-t-md border border-b-0 px-3 py-1.5 font-mono text-xs whitespace-nowrap ${
                i === active
                  ? "border-code-line border-t-accent bg-code-bg text-code-ink"
                  : "border-transparent bg-transparent text-code-soft hover:text-code-ink"
              }`}
            >
              <Blades size={11} />
              {f.path}
            </button>
          ))}
        </div>
        <span className="shrink-0 self-center py-1.5">
          <CopyButton text={files[active].code} />
        </span>
      </div>
      {bodies.map((body, i) => (
        <div key={i} id={`${id}-file-${i}`} role="tabpanel" aria-labelledby={`${id}-tab-${i}`} hidden={i !== active}>
          {body}
        </div>
      ))}
      {terminal}
      {statuses[active]}
    </>
  );
}
