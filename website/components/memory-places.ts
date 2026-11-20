import type { Frame, Obj } from "./memory-scenarios";

// Where everything in a memory step-through sits. Memory is drawn in
// address order, and every object, buffer and stack cell keeps one place
// for the whole scenario: the same position and the same address in every
// frame. An object not made yet is free cells at its place; a freed one
// goes grey where it is, then is free cells again. Nothing moves to fill a
// gap. Only a real move by the runtime may change an address, and no
// scenario here shows one: whether a growing buffer moves isn't drawn.

/** An example address, as the pictures write it. */
export const hex = (n: number) => `0x${n.toString(16).padStart(4, "0")}`;

/** Object `#n`'s fixed address: 0x1000, 0x1100, 0x1200… */
export const base = (id: string) => 0x1000 + (Number(id.slice(1)) - 1) * 0x100;

/** Where an object's item buffer or entries start, after its header. */
export const bufferAt = (id: string) => base(id) + 0x60;

/** Bytes a cell of an object's buffer takes: 16 for a dict's or set's entries, 8 for list items. */
export const cellBytes = (o: Obj) => (o.kind === "dict" || o.kind === "set" ? 16 : 8);

/** The example address of the function's first stack cell. */
export const STACK = 0x7ffde000;

/**
 * Every object the scenario ever has, in address order, each at its
 * biggest: the shape its place is drawn with in every frame.
 */
export function places(frames: Frame[]): Obj[] {
  const biggest = new Map<string, Obj>();
  for (const f of frames) {
    for (const o of [...f.objs, ...(f.freed ?? [])]) {
      const had = biggest.get(o.id);
      if (!had || o.cells.join("").length + o.cells.length > had.cells.join("").length + had.cells.length) biggest.set(o.id, o);
    }
  }
  return [...biggest.values()].sort((a, b) => base(a.id) - base(b.id));
}

/** Every name the function ever has, in the order it first appears: one fixed stack cell each. */
export function stackNames(frames: Frame[]): string[] {
  const out: string[] = [];
  for (const f of frames) for (const n of f.names) if (!out.includes(n.name)) out.push(n.name);
  return out;
}

/**
 * Fails the build when a scenario would draw something moving: an object
 * whose kind changes between frames (its layout, so its place, would
 * change), a buffer running into the next object, or a name that points
 * at an object the frame doesn't draw.
 */
export function checkPlaces(name: string, frames: Frame[]) {
  const all = places(frames);
  for (const f of frames) {
    for (const o of [...f.objs, ...(f.freed ?? [])]) {
      const p = all.find((q) => q.id === o.id)!;
      if (p.kind !== o.kind) throw new Error(`memory/step-${name}: ${o.id} is a ${p.kind} in one frame and a ${o.kind} in another`);
    }
    for (const n of f.names) {
      if (n.to && !n.ended && !f.objs.some((o) => o.id === n.to)) throw new Error(`memory/step-${name}: ${n.name} holds ${n.to}, which the frame at line ${f.line} doesn't have`);
    }
  }
  all.forEach((o, i) => {
    const next = all[i + 1];
    const end = bufferAt(o.id) + cellBytes(o) * Math.max(o.cells.length, 1);
    if (next && end > base(next.id)) throw new Error(`memory/step-${name}: ${o.id}'s cells run into ${next.id} at ${hex(base(next.id))}`);
  });
}
