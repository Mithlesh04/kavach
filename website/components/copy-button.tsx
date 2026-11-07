"use client";

import { useState } from "react";

export function CopyButton({ text, solid = false }: { text: string; solid?: boolean }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {}
  };
  const base = "rounded-md text-xs font-sans px-2.5 py-1 cursor-pointer";
  const look = solid
    ? "border-0 bg-inv-bg font-bold text-inv-ink px-3.5 py-1.5"
    : "border border-code-border bg-transparent text-code-ink";
  return (
    <button onClick={copy} className={`${base} ${look}`} aria-label={solid ? "Copy the commands" : "Copy the code"}>
      {/* aria-live: a screen reader hears "Copied" too, not only sees it. */}
      <span aria-live="polite">{copied ? "Copied ✓" : solid ? "Copy commands" : "Copy"}</span>
    </button>
  );
}
