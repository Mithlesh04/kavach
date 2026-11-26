"use client";

import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";

// Run on a sample: the code shows first, and Run shows what running it
// prints. That output was checked when the site was built (check-samples
// runs every sample against the real kavach), so Run reveals it, after
// the moment a run takes. It stays in the page's HTML all along, so search,
// crawlers and AI tools read it, and readers without JavaScript see it.

type Phase = "idle" | "running" | "done";
const Run = createContext<{ phase: Phase; start: () => void }>({ phase: "done", start: () => {} });

/** How long a run seems to take before its output shows, unless the reader asks for less motion. */
const RUN_MS = 450;

export function RunScope({ children }: { children: ReactNode }) {
  const [phase, setPhase] = useState<Phase>("idle");
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const start = () => {
    clearTimeout(timer.current);
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return setPhase("done");
    setPhase("running");
    timer.current = setTimeout(() => setPhase("done"), RUN_MS);
  };
  return <Run.Provider value={{ phase, start }}>{children}</Run.Provider>;
}

export function RunButton() {
  const { phase, start } = useContext(Run);
  return (
    <button
      onClick={start}
      disabled={phase === "running"}
      aria-label={phase === "idle" ? "Run the code" : "Run the code again"}
      className="run-action flex cursor-pointer items-center gap-1.5 rounded-md border-0 bg-accent px-2.5 py-1 font-sans text-xs font-bold text-bg disabled:cursor-wait disabled:opacity-70"
    >
      <svg width="9" height="10" viewBox="0 0 9 10" aria-hidden="true">
        <path d="M0 0 9 5 0 10z" fill="currentColor" />
      </svg>
      {phase === "idle" ? "Run" : "Run again"}
    </button>
  );
}

/**
 * The terminal under the code: hidden until Run, then the command at once
 * and what it printed after the run's moment. `command` is the line the
 * terminal shows while it runs.
 */
export function RunOutput({ children, command }: { children: ReactNode; command?: string }) {
  const { phase } = useContext(Run);
  return (
    <div className="run-output" hidden={phase === "idle"} aria-live="polite" aria-busy={phase === "running"}>
      {phase === "running" ? (
        <div className="border-t border-code-line bg-[var(--term-bg)] py-3 pl-[3.25rem] pr-4 font-mono text-[13px] leading-[1.7] max-sm:text-[12.5px]">
          {command && (
            <p className="m-0">
              <span aria-hidden="true" className="select-none font-bold text-[var(--term-green)]">$ </span>
              <span className="font-semibold text-code-ink">{command}</span>
            </p>
          )}
          <p className="m-0 animate-pulse text-code-soft">running…</p>
        </div>
      ) : (
        children
      )}
    </div>
  );
}

/** Opens the code in the playground, where the real compiler checks every change. */
export function EditButton({ code }: { code: string }) {
  const open = () => {
    const bytes = new TextEncoder().encode(code);
    let text = "";
    for (const b of bytes) text += String.fromCharCode(b);
    const packed = btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
    const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";
    window.open(`${base}/playground/#code=${packed}`, "_blank", "noopener");
  };
  return (
    <button
      onClick={open}
      aria-label="Edit the code in the playground"
      className="run-action cursor-pointer rounded-md border border-code-border bg-transparent px-2.5 py-1 font-sans text-xs text-code-ink max-sm:hidden"
    >
      Edit
    </button>
  );
}
