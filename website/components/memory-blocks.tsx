"use client";

import { CELL_PANEL, RunView, type Cell, type Run } from "./explain/cells";
import { CodeLines, Printed } from "./explain/code-track";
import { RamStick } from "./explain/pictures-storage";
import { STACK, base, bufferAt, cellBytes, hex, places, stackNames } from "./memory-places";
import type { Frame, Obj } from "./memory-scenarios";
import { StepControls, useSteps } from "./step-player";

// One picture of memory per step, drawn as memory looks: a RAM stick, one
// chip lit, and that chip's cells below it, 8 bytes each. The stack's cells
// hold the function's names; the heap's hold the objects, a count first.
// Every cell keeps one place and one address for the whole scenario
// (memory-places.ts): an object not made yet is free cells, a freed one goes
// grey where it is, and nothing moves to fill a gap. The data is in
// memory-scenarios.ts; memory-steps.tsx checks it against a sample.
// Addresses are examples: they differ on every run. The stick, the cells
// and the code beside them come from the explainer kit (explain/).

const COLORS: Record<string, string> = { "#1": "#e6a23c", "#2": "#5fbf7f", "#3": "#c586c0" };

type State = "live" | "freed" | "free";

/** Text as the words it fills inside its object: 8 bytes a word. */
const words = (s: string) => (s.match(/[\s\S]{1,8}/g) ?? [""]).map((w) => w.replace(/ /g, "␣"));

/** `xs` padded with free cells to `n`, so a place keeps its size in every frame. */
const pad = (xs: Cell[], n: number, addr: (i: number) => string): Cell[] =>
  Array.from({ length: Math.max(n, xs.length, 1) }, (_, i) => xs[i] ?? { text: "·", addr: addr(i), dim: true });

/**
 * The cells of one object at its fixed place, and of its item buffer or
 * entries when it has one. `shape` is the object at its biggest, which
 * sets how many cells the place has; `o` is what is there this frame.
 */
function objRuns(o: Obj, shape: Obj, state: State, before?: number): Run[] {
  const live = state === "live";
  const tint = state === "free" ? undefined : COLORS[o.id];
  const at = base(o.id);
  const lit = live && o.changed;
  const dim = !live;
  const title = state === "free" ? `free memory at ${hex(at)}` : `${o.id} ${o.kind === "shared" ? "Shared" : o.kind}`;
  const blank = (c: Cell): Cell => (state === "free" ? { ...c, text: "·", note: undefined, lit: false, flash: false } : c);
  const head: Cell = { text: state === "live" ? `count ${o.count}` : "count 0", note: "header, 24 B", flash: live && before !== undefined && before !== o.count };
  const cells = (xs: Cell[]) => xs.map((c, i) => blank({ ...c, addr: hex(at + 8 * (i === 0 ? 0 : i + 2)), tint, dim }));
  const freed = state === "freed";
  if (o.kind === "list" || o.kind === "dict" || o.kind === "set") {
    const buf = bufferAt(o.id);
    const step = cellBytes(o);
    const items = o.cells.map((x) => ({ text: x, tint, lit, dim }));
    const second: Cell = o.kind === "list" ? { text: `len ${o.cells.length}` } : { text: o.index ? "index ▦" : "no index", lit: lit && o.index };
    return [
      { title, tint, freed, cells: cells([head, { text: `→ ${hex(buf)}`, note: o.kind === "list" ? "items at" : "entries at" }, second]) },
      {
        title: state === "free" ? `free memory at ${hex(buf)}` : `${o.id} ${o.kind === "list" ? "items" : "entries, in order"}`,
        tint,
        freed,
        cells: pad(items, shape.cells.length, (i) => hex(buf + step * i)).map((c, i) => blank({ ...c, addr: hex(buf + step * i) })),
      },
    ];
  }
  if (o.kind === "text") {
    const s = o.cells.join("");
    const n = 2 + words(shape.cells.join("")).length;
    const xs = cells([head, { text: `len ${s.length}` }, ...words(s).map((w) => ({ text: w, lit }))]);
    return [{ title, tint, freed, cells: pad(xs, n, (i) => hex(at + 8 * (i + 2))) }];
  }
  return [{ title, tint, freed, cells: cells([head, { text: `→ ${hex(base(o.cells[0]))}`, note: "holds" }]) }];
}

/** Memory at frame `i`: the stack's fixed cells, then every object's place, in address order. */
function runsOf(frames: Frame[], i: number): Run[] {
  const f = frames[i];
  const prev = frames[i - 1];
  const stack: Run = {
    title: "stack: the function's names",
    cells: stackNames(frames).map((name, k) => {
      const addr = hex(STACK + 8 * k).replace("0x7ffd", "0x7ffd_");
      const n = f.names.find((x) => x.name === name);
      if (!n) return { text: "·", addr, note: "free", dim: true };
      return {
        text: n.ended ? "—" : n.value ?? `→ ${hex(base(n.to!))}`,
        addr,
        note: n.ended ? `${n.name} ended` : n.name,
        tint: n.to && !n.ended ? COLORS[n.to] : undefined,
        dim: n.ended,
        flash: n.value !== undefined && prev?.names.find((p) => p.name === n.name)?.value !== n.value,
      };
    }),
  };
  const heap = places(frames).flatMap((p) => {
    const live = f.objs.find((o) => o.id === p.id);
    if (live) return objRuns(live, p, "live", prev?.objs.find((o) => o.id === p.id)?.count);
    const gone = f.freed?.find((o) => o.id === p.id);
    return gone ? objRuns(gone, p, "freed") : objRuns(p, p, "free");
  });
  return [stack, ...heap];
}

/** Each name's last use, by line: the line of the first frame where the name has ended. */
function lastUses(frames: Frame[]): Record<number, string[]> {
  const out: Record<number, string[]> = {};
  const seen = new Set<string>();
  for (const f of frames) {
    for (const n of f.names) {
      if (!n.ended || seen.has(n.name)) continue;
      seen.add(n.name);
      (out[f.line] ??= []).push(n.name);
    }
  }
  return out;
}

/** Steps through `memory/step-<name>.kav`: the line, and memory's cells at it. */
export function MemoryBlocks({ frames, html, output }: { frames: Frame[]; html: string[]; output: string }) {
  const steps = useSteps(frames.length, 2200);
  const f = frames[steps.frame];
  const runs = runsOf(frames, steps.frame);
  return (
    <div className="flex min-w-0 flex-col gap-4 rounded-xl border border-line bg-panel px-4 py-4 sm:px-5" role="group" aria-label="Memory, step by step">
      <div className="grid grid-cols-[minmax(0,1fr)] gap-4 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
        <div className="flex min-w-0 flex-col gap-2">
          <CodeLines html={html} line={f.line} lastUse={lastUses(frames)} />
          <Printed text={output} label="The program prints, checked:" />
        </div>
        <div className="flex min-w-0 flex-col gap-2">
          <RamStick />
          <div className={CELL_PANEL}>
            {runs.map((r, i) => <RunView key={i} run={r} />)}
            {runs.length === 1 && <span className="text-[12.5px] text-[#9aa0a6]">heap: no objects here</span>}
          </div>
          <span className="text-[11px] text-soft">Each cell is 8 bytes. Addresses are examples: they differ on every run.</span>
        </div>
      </div>
      <StepControls steps={steps} caption={f.caption} />
    </div>
  );
}
