import { CELL_PANEL, RunView } from "./cells";
import type { CallFrame, HeapObject, Slot } from "./types";

// The panels beside a scene's board: the registers, the call stack and the
// heap. Each is drawn as memory looks, in 8-byte cells on a dark panel.
// The timeline is in timeline.tsx.

export const hex = (n: number) => `0x${n.toString(16).padStart(12, "0").replace(/^(.{4})(.{4})(.{4})$/, "$1_$2_$3")}`;

/** An example stack address: the stack starts high and grows down. */
const STACK_TOP = 0x7ffd_e000_0f00;

/** The example address of the stack's slot with `above` slots above it. */
export const stackAddr = (above: number) => STACK_TOP - 8 * (above + 1);

function SlotCell({ slot, changed, addr, lit }: { slot: Slot; changed?: boolean; addr?: string; lit?: boolean }) {
  return (
    <div className="grid grid-cols-[minmax(0,auto)_minmax(0,1fr)] items-center gap-x-2 gap-y-0.5">
      <span
        className={`flex h-8 min-w-[64px] items-center justify-center rounded-[3px] border px-1.5 font-mono text-[12px] font-semibold text-[#e8e6e1] ${changed ? "mem-flash" : ""}`}
        style={{ background: lit ? "color-mix(in srgb, #e6a23c 24%, #101418)" : "#101418", borderColor: lit ? "#e6a23c" : "#2f3640" }}
      >
        {slot.value}
      </span>
      <span className="flex min-w-0 flex-col leading-tight">
        <span className="font-mono text-[11.5px] font-bold text-[#e8e6e1]">{slot.name}</span>
        {slot.note && <span className="text-[10.5px] text-[#b5aea2]">{slot.note}</span>}
        {addr && <span className="font-mono text-[9.5px] text-[#8f887b]">{addr}</span>}
      </span>
    </div>
  );
}

function PanelTitle({ children, hint }: { children: React.ReactNode; hint?: string }) {
  return (
    <span className="flex flex-wrap items-baseline gap-x-2 text-[12px] font-bold text-[#e8e6e1]">
      {children}
      {hint && <span className="text-[10.5px] font-normal text-[#9aa0a6]">{hint}</span>}
    </span>
  );
}

/** The registers that matter this step; a changed value flashes. */
export function RegistersPanel({ regs, before }: { regs: Slot[]; before?: Slot[] }) {
  return (
    <div className={CELL_PANEL} role="group" aria-label="Registers">
      <PanelTitle hint="inside the core, 8 bytes each">Registers</PanelTitle>
      {regs.map((r) => (
        <SlotCell key={r.name} slot={r} changed={before?.find((b) => b.name === r.name)?.value !== r.value} />
      ))}
    </div>
  );
}

/**
 * The call stack, oldest call at the top as in memory: each new frame sits
 * below, at lower addresses. A frame that just returned shows faded.
 */
export function CallStackPanel({ stack, before }: { stack: CallFrame[]; before?: CallFrame[] }) {
  const gone = (before ?? []).slice(stack.length);
  const all = [...stack, ...gone];
  /** How many slots sit above frame i. */
  const above = (i: number) => all.slice(0, i).reduce((n, f) => n + f.slots.length, 0);
  const frameView = (f: CallFrame, i: number, popped: boolean) => {
    const newest = !popped && i === stack.length - 1;
    return (
      <div
        key={`${popped ? "gone" : "live"}-${i}-${f.fn}`}
        className={`flex flex-col gap-1.5 rounded-md border p-2 ${popped ? "border-dashed border-[#454b55] opacity-45" : newest ? "mem-in border-[#e6a23c]" : "border-[#2f3640]"}`}
      >
        <span className="font-mono text-[12px] font-bold text-[#e8e6e1]">
          {f.fn}
          {popped && <span className="font-sans font-normal text-[#7fc98f]"> · returned, frame popped</span>}
        </span>
        {f.slots.map((s, j) => {
          const addr = stackAddr(above(i) + j);
          const old = before?.[i]?.fn === f.fn ? before[i].slots.find((b) => b.name === s.name)?.value : s.value;
          return <SlotCell key={s.name} slot={s} addr={hex(addr)} changed={!popped && old !== s.value} lit={newest} />;
        })}
      </div>
    );
  };
  return (
    <div className={CELL_PANEL} role="group" aria-label="The call stack">
      <PanelTitle hint="in RAM; grows down ↓">Call stack</PanelTitle>
      {stack.map((f, i) => frameView(f, i, false))}
      {gone.map((f, i) => frameView(f, stack.length + i, true))}
    </div>
  );
}

/**
 * The heap's objects, in the memory step-through's cell look; with a
 * `title` and `hint`, another memory's: the GPU's buffers in its VRAM.
 */
export function HeapPanel({ heap, empty, title = "Heap", hint = "in RAM" }: { heap: HeapObject[]; empty: string; title?: string; hint?: string }) {
  return (
    <div className={CELL_PANEL} role="group" aria-label={title === "Heap" ? "The heap" : title}>
      <PanelTitle hint={hint}>{title}</PanelTitle>
      {heap.length === 0 && <span className="text-[12px] leading-snug text-[#9aa0a6]">{empty}</span>}
      {heap.map((o) => (
        <RunView key={o.title} run={{ title: o.title, tint: o.tint, freed: o.freed, cells: o.cells.map((c) => ({ ...c, tint: o.tint })) }} />
      ))}
    </div>
  );
}
