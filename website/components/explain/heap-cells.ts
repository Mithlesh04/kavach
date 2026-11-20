import type { HeapObject, SceneFrame } from "./types";

// The heap as kit scenes draw it, the way the memory step-through does
// (../memory-places.ts): objects and buffers at fixed example addresses,
// short like 0x1000, in address order. A scene lists every place it ever
// uses in every frame: a place not made yet is free memory, a freed one
// goes grey where it is, and nothing moves to fill a gap. `checkHeap`
// fails the build when two frames draw a place at different addresses.

/** A cell as a scene writes it: its text, or the text with how it looks. */
export type CellIn = string | { text: string; lit?: boolean; dim?: boolean; note?: string };

/** Fixed colors for objects, as the memory step-through uses: the cells sit on a dark panel in both themes. */
export const TINT = { a: "#e6a23c", b: "#5fbf7f", c: "#c586c0", d: "#6fb3e0" };

/** An example address. */
export const addr = (n: number) => `0x${n.toString(16).padStart(4, "0")}`;

/** An object or buffer at `base`, its cells `step` bytes apart (8 unless said). */
export function place(title: string, base: number, cells: CellIn[], opts: { step?: number; freed?: boolean; tint?: string } = {}): HeapObject {
  const step = opts.step ?? 8;
  return {
    title,
    tint: opts.freed ? undefined : opts.tint,
    freed: opts.freed,
    cells: cells.map((c, i) => ({ ...(typeof c === "string" ? { text: c } : c), addr: addr(base + i * step) })),
  };
}

/** Free memory at `base`: `n` cells nothing holds yet. */
export const freeAt = (base: number, n: number, step = 8) =>
  place(`free memory at ${addr(base)}`, base, Array.from({ length: n }, () => ({ text: "·", dim: true })), { step });

/**
 * The 24-byte header every object starts with, as three 8-byte words: the
 * count, then the object's kind and its type (runtime/native/src/heap.rs, Obj).
 */
export const header = (count: number): CellIn[] => [
  { text: `count ${count}`, note: "header" },
  { text: "kind", note: "header" },
  { text: "type", note: "header" },
];

/** Room a buffer has but doesn't use yet: allocated, empty. */
export const ROOM = { text: "·", note: "room" };

/** A free cell inside a place: memory past what the buffer has now. */
export const FREE = { text: "·", dim: true };

/** `xs` padded with free cells to `n`, so a buffer keeps its size in every frame. */
export const padded = (xs: CellIn[], n: number): CellIn[] => [...xs, ...Array.from({ length: Math.max(0, n - xs.length) }, () => FREE)];

/**
 * Fails the build unless every frame draws the same places: the same
 * cells at the same addresses, in the same order. So no object, buffer or
 * cell can look as if it moved.
 */
export function checkHeap(id: string, frames: SceneFrame[]) {
  const shape = (f: SceneFrame) => (f.heap ?? []).map((o) => o.cells.map((c) => c.addr).join(" ")).join(" | ");
  const first = shape(frames[0]);
  frames.forEach((f, i) => {
    if (shape(f) !== first) throw new Error(`scene "${id}": step ${i + 1} draws the heap's places differently from step 1 (components/explain/heap-cells.ts)`);
  });
}
