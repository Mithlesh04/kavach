"use client";

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

type Shown = { el: HTMLElement; chip: string; note: string; kind: string };

const GAP = 10;
const EDGE = 8;

/**
 * The hover notes on code (`.tip` spans that CodeBlock adds), drawn in one
 * floating box attached to <body>, so a code panel that scrolls can never
 * clip them. It opens on hover or keyboard focus, sits below the word when
 * there is room and above it when not, stays inside the window, follows
 * the word as anything scrolls, and closes on Escape.
 */
export function TipLayer() {
  const [shown, setShown] = useState<Shown | null>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const tipOf = (t: EventTarget | null) => (t instanceof Element ? (t.closest(".tip") as HTMLElement | null) : null);
    const open = (el: HTMLElement) =>
      setShown({
        el,
        chip: el.dataset.chip ?? "",
        note: el.dataset.note ?? "",
        kind: [...el.classList].find((c) => c.startsWith("tip-"))?.slice(4) ?? "new",
      });
    const over = (e: Event) => {
      const el = tipOf(e.target);
      if (el) open(el);
    };
    const out = (e: Event) => {
      const from = tipOf(e.target);
      const to = tipOf((e as PointerEvent | FocusEvent).relatedTarget);
      if (from && from !== to) setShown((s) => (s?.el === from ? null : s));
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setShown(null);
    };
    document.addEventListener("pointerover", over);
    document.addEventListener("pointerout", out);
    document.addEventListener("focusin", over);
    document.addEventListener("focusout", out);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerover", over);
      document.removeEventListener("pointerout", out);
      document.removeEventListener("focusin", over);
      document.removeEventListener("focusout", out);
      document.removeEventListener("keydown", key);
    };
  }, []);

  // Place the box next to its word, and keep it there while anything scrolls.
  useLayoutEffect(() => {
    if (!shown) return;
    const place = () => {
      const r = shown.el.getBoundingClientRect();
      const b = box.current?.getBoundingClientRect();
      const w = b?.width ?? 300;
      const h = b?.height ?? 80;
      const below = r.bottom + GAP + h <= window.innerHeight - EDGE || r.top - GAP - h < EDGE;
      setPos({
        left: Math.min(Math.max(r.left - 6, EDGE), window.innerWidth - w - EDGE),
        top: below ? r.bottom + GAP : r.top - GAP - h,
      });
    };
    place();
    shown.el.setAttribute("aria-describedby", "code-tip");
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      shown.el.removeAttribute("aria-describedby");
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  }, [shown]);

  if (!shown) return null;
  const [what, why] = shown.note.split(" — Why: ");
  return createPortal(
    <div
      ref={box}
      id="code-tip"
      role="tooltip"
      className="pointer-events-none fixed z-50 w-[300px] max-w-[calc(100vw-16px)] rounded-[10px] border border-line bg-panel px-[13px] py-[11px] text-[12.5px] leading-[1.55] text-ink shadow-[0_12px_28px_rgba(0,0,0,0.25)]"
      style={{ left: pos?.left ?? -9999, top: pos?.top ?? -9999 }}
    >
      <span className={`mb-1 block text-[10.5px] font-extrabold uppercase tracking-wider tip-chip-${shown.kind}`}>{shown.chip}</span>
      <span className="block">{what}</span>
      {why ? (
        <span className="mt-1 block text-muted">
          <strong className="font-semibold text-ink">Why:</strong> {why}
        </span>
      ) : null}
    </div>,
    document.body,
  );
}
