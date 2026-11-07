"use client";

import { useState } from "react";
import versions from "@/data/versions.json";

export function VersionPicker() {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        aria-haspopup="listbox"
        aria-expanded={open}
        className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-face px-[11px] py-1.5 font-mono text-[12.5px] font-bold text-ink max-lg:h-10"
      >
        {versions.latest} <span className="font-medium text-soft max-sm:hidden">latest</span>
        <svg width="10" height="6" viewBox="0 0 10 6" aria-hidden="true">
          <path d="m1 1 4 4 4-4" stroke="currentColor" strokeWidth="1.6" fill="none" />
        </svg>
      </button>
      {open && (
        <div
          role="listbox"
          aria-label="Documentation versions"
          className="absolute left-0 top-[46px] z-40 w-80 rounded-[10px] border border-line bg-panel p-2 shadow-[0_10px_28px_rgba(0,0,0,0.14)] max-sm:fixed max-sm:inset-x-3 max-sm:top-[64px] max-sm:w-auto"
        >
          {versions.versions.map((v) => (
            <div key={v.id} className={`rounded-lg px-[11px] py-[9px] ${v.id === versions.latest ? "bg-active" : ""}`}>
              <p className="m-0 text-[13.5px] font-extrabold text-ink">
                {v.id}
                {v.id === versions.latest && <span className="font-semibold text-muted"> · latest</span>}
              </p>
              <p className="m-0 mt-0.5 text-xs text-soft">every file:line link pinned to tag {v.tag}</p>
            </div>
          ))}
          <div className="mx-1 mb-0.5 mt-1.5 border-t border-line-2 px-[7px] pt-2">
            <p className="m-0 text-[11.5px] leading-normal text-soft">
              A retired version&apos;s links redirect here with a notice. Versions are added and deleted by one
              entry in versions.json.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
