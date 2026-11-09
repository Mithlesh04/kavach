"use client";

import { useState } from "react";

type Mode = "auto" | "light" | "dark";
const next: Record<Mode, Mode> = { auto: "light", light: "dark", dark: "auto" };

/** Auto follows the browser; a forced choice is remembered per visitor. */
export function ThemeToggle() {
  // The pre-paint script in layout.tsx already set the forced theme on
  // <html>, so the initial state reads it from there — no effect needed.
  const [mode, setMode] = useState<Mode>(() => {
    if (typeof document === "undefined") return "auto";
    const t = document.documentElement.dataset.theme;
    return t === "light" || t === "dark" ? t : "auto";
  });

  const cycle = () => {
    const m = next[mode];
    setMode(m);
    try {
      if (m === "auto") localStorage.removeItem("kavach-theme");
      else localStorage.setItem("kavach-theme", m);
    } catch {}
    if (m === "auto") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = m;
  };

  const label = mode === "auto" ? "Auto" : mode === "light" ? "Light" : "Dark";
  return (
    <button
      onClick={cycle}
      aria-label="Theme: follows your browser; click to force light or dark"
      className="flex shrink-0 items-center gap-2 rounded-lg border border-line bg-face px-3 py-[7px] text-[12.5px] font-bold text-muted cursor-pointer max-lg:h-10 max-lg:w-10 max-lg:justify-center max-lg:p-0"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true" className="max-lg:h-[18px] max-lg:w-[18px]">
        <circle cx="12" cy="12" r="4.4" stroke="currentColor" strokeWidth="2" />
        <path
          d="M12 2.5v2.4M12 19.1v2.4M2.5 12h2.4M19.1 12h2.4M4.9 4.9l1.7 1.7M17.4 17.4l1.7 1.7M19.1 4.9l-1.7 1.7M6.6 17.4l-1.7 1.7"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>
      <span suppressHydrationWarning className="max-lg:hidden">{label}</span>
    </button>
  );
}
