import { TINT, addr, checkHeap, freeAt, header, place } from "./heap-cells";
import type { HeapObject, PipelineView, Scene, SceneFrame, Slot } from "./types";

// map → filter → sum, as written and fused: the sample
// explain/chain.native.kav. Checked in the source (2026-11-21):
// - A lambda written in map, filter or sum is inlined into a loop: no
//   call per item, no closure object (compiler/kir/src/lower_lambda.rs,
//   5-8 and LIST_LOOPS 21-33).
// - The fuse pass (compiler/kir/src/fuse.rs, 5-19) hoists a chain into
//   statements, then fuses a loop that builds a list used only by the next
//   loop into it. The fused loop is the fast block of a Fallback: when a
//   check in it fails, the code as written runs instead. KAVACH_FUSE_LOG=1
//   prints "fuse: main line 3" for this sample.
// - Only LLVM runs the fast block (compiler/backend/llvm/src/expr.rs:331);
//   Cranelift, `kavach run --native`, runs the code as written
//   (compiler/backend/cranelift/src/codegen.rs:435-437).
// - Every * and + on Int is checked for overflow (cranelift/src/ops.rs,
//   78-91; llvm/src/ops.rs, 31-40).
// - Heap locals are released when their function returns, and main skips
//   that unless leaks are checked, in Cranelift and LLVM alike
//   (cranelift/src/codegen.rs, 137-169; llvm/src/func.rs, 339-346): the
//   lists in between, held by the loops' locals, stay to the end, as drawn.
//   The interpreter frees everything when main ends (interp lib.rs:286-300).
// Whether map's list gets its whole room at once is not checked: the
// lists in between are drawn without their capacity.

const core = "cpu.core0";
const chip = "ram.chip2";

const STAGES = ["map: p * 2", "filter: p > 100", "sum"];

type State = "none" | "live";

/** prices, map's list and filter's list, each at its own place in every step. */
function heap(mapped: State, kept: State, built = true): HeapObject[] {
  /** Free memory; in the fused program, memory the lists in between never need. */
  const free = (base: number, n: number) => (built ? freeAt(base, n) : { ...freeAt(base, n), title: `${freeAt(base, n).title}: never needed in this build` });
  const obj = (title: string, base: number, buf: number, state: State, len: number) =>
    state === "none"
      ? free(base, 5)
      : place(title, base, [...header(1), { text: `→ ${addr(buf)}`, note: "ptr" }, `len ${len}`], { tint: base === 0x1000 ? TINT.a : base === 0x1030 ? TINT.b : TINT.c });
  const items = (title: string, buf: number, state: State, xs: string[], n: number) =>
    state === "none" ? free(buf, n) : place(title, buf, xs, { tint: buf === 0x2040 ? TINT.b : TINT.c });
  return [
    obj("prices: a list object", 0x1000, 0x2000, "live", 4),
    obj("map's new list", 0x1030, 0x2040, mapped, 4),
    obj("filter's new list", 0x1060, 0x2080, kept, 3),
    place("prices: its items", 0x2000, ["120", "40", "300", "75"], { tint: TINT.a }),
    items("map's items", 0x2040, mapped, ["240", "80", "600", "150"], 4),
    items("filter's items", 0x2080, kept, ["240", "600", "150"], 3),
  ];
}

const pipe = (items: { label: string; stage: number }[], notes?: Record<number, string>): PipelineView => ({ title: "The loops, as the code runs them", stages: STAGES, items, notes });
const regs = (...r: [string, string, string?][]): Slot[] => r.map(([name, value, note]) => ({ name, value, note }));
const step = (line: number, text: string, extra: Partial<SceneFrame> & { caption: string; heap: HeapObject[] }): SceneFrame => ({ line, at: text, lit: [core, chip], ...extra });

const frames: SceneFrame[] = [
  step(0, "const prices", { heap: heap("none", "none"), caption: "Four prices: one list object and its buffer." }),
  step(1, ".map(p => p * 2)", {
    labels: { [core]: "map's loop" },
    heap: heap("live", "none"),
    pipeline: pipe(["240", "80", "600", "150"].map((label) => ({ label, stage: 0 }))),
    caption:
      "kavach run --native (Cranelift) runs the chain as written. The compiler, while compiling, wrote each lambda straight into a loop: no call per item, no closure object. First map's loop builds a new list: each price times 2, each multiply checked for overflow.",
  }),
  step(1, ".filter(p => p > 100)", {
    labels: { [core]: "filter's loop" },
    heap: heap("live", "live"),
    pipeline: pipe(["240", "600", "150"].map((label) => ({ label, stage: 1 })), { 1: "80 left out" }),
    caption:
      "Then filter's loop reads that list and builds another: 240, 600 and 150 pass p > 100, and 80 doesn't. Nothing reads map's list after this, but filter's loop keeps it in a local of main. In native code, Cranelift or LLVM alike, main lets go of nothing its locals still hold: the operating system takes that memory back when the program ends. The compiler decided that while compiling.",
  }),
  step(1, ".sum(p => p)", {
    labels: { [core]: "sum's loop" },
    regs: regs(["total", "990", "a register: no list"]),
    heap: heap("live", "live"),
    pipeline: pipe([{ label: "990", stage: 3 }]),
    caption: "Then sum's loop adds the items up in a register, each + checked for overflow. No list this time. Two lists were built on the way, each written item by item and read once.",
  }),
  step(2, "print(total)", { lit: [core, "kernel.files"], labels: { "kernel.files": "output" }, heap: heap("live", "live"), output: "990", caption: "990 is printed." }),
  step(1, ".map(p => p * 2)", {
    labels: { [core]: "one loop" },
    regs: regs(["p", "120 → 240"], ["total", "240"]),
    heap: heap("none", "none", false),
    pipeline: pipe([{ label: "240", stage: 2 }]),
    caption:
      "The same line in a program built with kavach build (LLVM). While compiling, the compiler saw that each list in between is read only by the next loop, and fused the three loops into one. Each price goes through map, then filter, then into the total, in registers: 120 × 2 = 240, kept, total 240.",
  }),
  step(1, ".filter(p => p > 100)", {
    labels: { [core]: "one loop" },
    regs: regs(["p", "40 → 80, then 300 → 600"], ["total", "840"]),
    heap: heap("none", "none", false),
    pipeline: pipe([{ label: "600", stage: 2 }], { 1: "80 left out" }),
    caption: "40 × 2 = 80 fails p > 100 and is left out at once; 300 × 2 = 600 is added: total 840. No item is written to memory.",
  }),
  step(1, ".sum(p => p)", {
    labels: { [core]: "one loop" },
    regs: regs(["p", "75 → 150"], ["total", "990"]),
    heap: heap("none", "none", false),
    pipeline: pipe([{ label: "990", stage: 3 }]),
    caption:
      "75 × 2 = 150 is added: 990, the same answer, and no list was ever built. A fused loop does the work in a new order, so a failed check could stop at another place. So the compiler keeps the code as written beside it: if a check in the fused loop fails, that code runs instead, and the panic names the right line.",
  }),
  step(2, "print(total)", { lit: [core, "kernel.files"], labels: { "kernel.files": "output" }, heap: heap("none", "none", false), output: "990", caption: "990 again: both ways print the same." }),
];

checkHeap("chain", frames);

export const chainScene: Scene = {
  id: "chain",
  title: "map, filter, sum: as written, and fused into one loop",
  caveman: "As written: map make list, filter make list, sum add. Fused: one loop, each item go all the way, no list in middle. Same answer.",
  sample: "explain/chain.native",
  code: "main",
  board: [
    { kind: "cpu", col: 0, row: 0, label: "CPU: core 0 runs the loops" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the lists" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  panels: ["regs", "heap"],
  lastUse: { 1: ["prices"] },
  spec: "14.2",
  frames,
  note: "Steps 1 to 5 are kavach run --native, which uses Cranelift and runs the code as written; steps 6 to 9 are a program built with kavach build, which uses LLVM and runs the fused loop. kavach run, the interpreter, frees everything when main ends. Addresses are examples.",
};
