"use client";

import { useEffect, useRef, useSyncExternalStore } from "react";

// A packet: a small labelled box that travels along a link of the board,
// moved by requestAnimationFrame along the link's own path. A reader who
// asks for less motion sees it already at the end of its trip.

const QUERY = "(prefers-reduced-motion: reduce)";

function subscribe(onChange: () => void) {
  const m = window.matchMedia(QUERY);
  m.addEventListener("change", onChange);
  return () => m.removeEventListener("change", onChange);
}

/** Whether the reader asked for less motion; false while the server renders. */
export function useReducedMotion() {
  return useSyncExternalStore(subscribe, () => window.matchMedia(QUERY).matches, () => false);
}

/** How long one trip takes, in milliseconds. */
const TRIP = 1100;

const ease = (t: number) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

/** A path between two points, curving out of the first and into the second. */
export function linkPath([ax, ay]: [number, number], [bx, by]: [number, number]) {
  if (Math.abs(bx - ax) >= Math.abs(by - ay)) {
    const mx = (ax + bx) / 2;
    return `M${ax} ${ay} C${mx} ${ay} ${mx} ${by} ${bx} ${by}`;
  }
  const my = (ay + by) / 2;
  return `M${ax} ${ay} C${ax} ${my} ${bx} ${my} ${bx} ${by}`;
}

/**
 * One packet's trip from `from` to `to`, starting after `delay` ms. It
 * stays at the end, so a reader can still read its label.
 */
export function Packet({ from, to, label, delay = 0 }: { from: [number, number]; to: [number, number]; label: string; delay?: number }) {
  const path = useRef<SVGPathElement>(null);
  const box = useRef<SVGGElement>(null);
  const still = useReducedMotion();
  const d = linkPath(from, to);
  useEffect(() => {
    const p = path.current;
    const g = box.current;
    if (!p || !g) return;
    const length = p.getTotalLength();
    const place = (t: number) => {
      const at = p.getPointAtLength(length * t);
      g.setAttribute("transform", `translate(${at.x} ${at.y})`);
    };
    if (still) {
      place(1);
      return;
    }
    place(0);
    let frame = 0;
    const start = performance.now() + delay;
    const tick = (now: number) => {
      const t = Math.min(1, Math.max(0, (now - start) / TRIP));
      place(ease(t));
      if (t < 1) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [d, still, delay]);
  const w = Math.max(24, label.length * 5.6 + 12);
  return (
    <g aria-hidden="true">
      <path ref={path} d={d} fill="none" stroke="none" />
      <g ref={box} transform={`translate(${from[0]} ${from[1]})`}>
        <rect x={-w / 2} y="-9" width={w} height="18" rx="9" className="fill-accent stroke-panel" strokeWidth="1.5" />
        <text y="3.5" textAnchor="middle" fontSize="9.5" fontWeight="800" className="fill-panel font-mono">
          {label}
        </text>
      </g>
    </g>
  );
}
