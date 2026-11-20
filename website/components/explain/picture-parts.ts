import type { ReactNode } from "react";

// What every resource picture shares. Hardware is drawn in its own fixed
// colors (a green circuit board, black chips, gold contacts), the same in
// both themes, so a lit part uses the fixed amber of the code panels.
// Words outside the hardware use the theme's tokens (fill-ink, fill-soft).

/** A picture's own box, in SVG units. The board gives each a cell around it. */
export const PIC = { w: 220, h: 150 };

/** Fixed hardware colors: real parts look the same in light and dark. */
export const HW = {
  lit: "#e6a23c",
  litFill: "#3a2a10",
  chip: "#141414",
  chipLine: "#2b2b2b",
  pcb: "#1f5a3a",
  pcbLine: "#123824",
  gold: "#d4a93c",
  ink: "#e8e6e1",
  soft: "#9aa0a6",
  metal: "#8d939b",
  die: "#22252b",
};

export type PicProps = {
  /** Whether a part (`core2`, `chip3`, `vram`) is lit this step. */
  on: (part: string) => boolean;
  /** The word shown on a part this step, if any. */
  tag: (part: string) => string | undefined;
  /** A CPU's cores. */
  cores?: number;
};

/** A resource picture: how to draw it, and where each of its parts is. */
export type Picture = {
  /** What a screen reader hears for it. */
  name: string;
  draw: (p: PicProps) => ReactNode;
  /** The middle of a part, or of the whole picture, in the picture's box. */
  anchor: (part: string | undefined, cores: number) => [number, number];
};

/** Stroke and fill of a hardware part, lit or not. */
export function partStyle(lit: boolean, fill: string, line: string) {
  return { fill: lit ? HW.litFill : fill, stroke: lit ? HW.lit : line, strokeWidth: lit ? 2.2 : 1 };
}

/** Cuts a word to fit a small part. */
export const fit = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);
