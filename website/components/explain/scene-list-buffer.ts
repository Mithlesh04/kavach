import { ROOM, TINT, checkHeap, freeAt, header, padded, place, type CellIn } from "./heap-cells";
import type { HeapObject, Scene, SceneFrame, Slot } from "./types";

// A list of whole numbers in memory, as native code keeps it: the sample
// explain/list-buffer.native.kav (`kavach run --native`, Cranelift).
// Checked in the source (2026-11-21):
// - A list is a 48-byte object made with Box: the 24-byte header (the
//   count first), then ptr, len and cap at offsets 24, 32 and 40
//   (runtime/native/src/heap.rs:42-47, ListObj at 108-114). Its items are
//   a separate buffer, a Rust Vec<u64> taken apart; Int items are 8 bytes
//   each, in the buffer itself.
// - A list written out gets exactly its items' room: kv_list ->
//   alloc_zeroed -> vec![0; n] (rt_data.rs:48-50, heap.rs:287-291).
// - xs[i]: one unsigned compare i < len, so a negative i fails too, then
//   one load at ptr + i * 8 (compiler/backend/cranelift/src/inline.rs,
//   check_index and item_at, 95-117).
// - push: inline, count == 1 and len < cap, then a store and len + 1;
//   otherwise the runtime's ListPush (inline.rs list_push, 292-330), which
//   pushes onto the Vec (runtime/native/src/lists.rs push_item, 147-163).
//   Rust 1.99.1 (rust-toolchain.toml) grows a Vec of 3 to 6 on a push:
//   checked with a test program. Whether the buffer moves is the
//   allocator's choice, not checked: the scene keeps its address and
//   says "may move".
// - A slice is a new value (spec 2.19): rt_slices.rs list_part (34-61)
//   copies the picked items into a new list of exactly that many.
// - list[i] = v: inline, count == 1 and i in range, stored in place
//   (inline.rs list_set, 332-345).
// - xs.swap(i, j): lowered to the runtime's ListSwap (compiler/kir/src/
//   lower_std.rs:233), runtime/native/src/lists.rs swap_checked (176-209):
//   both indices checked against len, `unique` copies only when something
//   else holds the list, then the two items' words change places. Nothing
//   is counted, text and records included (the maintainer's checker).
// Packed lists round their room up to whole 8-byte words (heap.rs:407-417):
// the note says so.

const core = "cpu.core0";
const xsChip = "ram.chip2";
const firstsChip = "ram.chip4";

type Xs = { count?: number; len: number; cap: number; items: CellIn[] };
type Firsts = "none" | "live" | "freed";

const XS = 0x1000;
const FIRSTS = 0x1030;
const XS_BUF = 0x2000;
const FIRSTS_BUF = 0x2040;

/** The heap: xs's object and buffer, firsts' object and buffer, always at their places. */
function heap(xs: Xs, firsts: Firsts, xsFreed = false, firstsItems: CellIn[] = ["3", "1"]): HeapObject[] {
  const xsObj = place("xs: a list object, 48 bytes", XS, [...header(xs.count ?? 1), { text: "→ 0x2000", note: "ptr" }, `len ${xs.len}`, `cap ${xs.cap}`], { tint: TINT.a, freed: xsFreed });
  const xsBuf = place("xs: its items, 8 bytes each", XS_BUF, padded(xs.items, 6), { tint: TINT.a, freed: xsFreed });
  const fObj =
    firsts === "none"
      ? freeAt(FIRSTS, 6)
      : place("firsts: a new list object", FIRSTS, [...header(firsts === "freed" ? 0 : 1), { text: "→ 0x2040", note: "ptr" }, "len 2", "cap 2"], { tint: TINT.b, freed: firsts === "freed" });
  const fBuf = firsts === "none" ? freeAt(FIRSTS_BUF, 2) : place("firsts: its own items", FIRSTS_BUF, firstsItems, { tint: TINT.b, freed: firsts === "freed" });
  return [xsObj, fObj, xsBuf, fBuf];
}

const lit = (text: string) => ({ text, lit: true });
const regs = (...r: [string, string, string?][]): Slot[] => r.map(([name, value, note]) => ({ name, value, note }));

const START: Xs = { len: 3, cap: 3, items: ["3", "1", "4"] };
const GROWN: Xs = { len: 4, cap: 6, items: ["3", "1", "4", lit("5"), ROOM, ROOM] };
const FIVE: Xs = { len: 5, cap: 6, items: ["3", "1", "4", "5", lit("9"), ROOM] };
const SEVEN: Xs = { len: 5, cap: 6, items: [lit("7"), "1", "4", "5", "9", ROOM] };
const SWAPPED: Xs = { len: 5, cap: 6, items: [lit("9"), "1", "4", "5", lit("7"), ROOM] };

const frames: SceneFrame[] = [
  {
    line: 0,
    at: "let xs = [3, 1, 4]",
    lit: [core, xsChip],
    labels: { [xsChip]: "xs" },
    heap: heap({ ...START, items: ["3", "1", "4"].map(lit) }, "none"),
    caption:
      "The runtime makes the list while running, in two pieces: a 48-byte list object (a 24-byte header with the count, then the buffer's address, the length and the capacity), and apart from it a buffer of exactly 3 slots. Whole numbers sit in the buffer itself, 8 bytes each: no object per item.",
  },
  {
    line: 1,
    at: "xs[1]",
    lit: [core, xsChip],
    labels: { [core]: "1 < 3?", [xsChip]: "xs" },
    packets: [{ from: xsChip, to: core, label: "1" }],
    regs: regs(["index", "1"], ["len", "3", "read from the object"], ["address", "0x2008", "ptr + 1 × 8"], ["result", "1"]),
    heap: heap({ ...START, items: ["3", lit("1"), "4"] }, "none"),
    output: "1",
    caption:
      "Reading xs[1]: code the compiler wrote checks the index with one compare, 1 < len 3. A negative index counts as a huge unsigned number here, so the same compare catches it too. Then one load from ptr + 1 × 8 = 0x2008 gives 1.",
  },
  {
    line: 2,
    at: "xs.push(5)",
    lit: [core, xsChip],
    labels: { [core]: "grow", [xsChip]: "xs" },
    regs: regs(["count", "1", "nobody else holds it"], ["len < cap", "3 < 3: no", "the buffer is full"]),
    heap: heap(GROWN, "none"),
    output: "1",
    caption:
      "push checks two things inline: the count is 1, and len < cap. 3 < 3 is false, so the code calls the runtime. The runtime, while running, grows the buffer the way Rust's Vec does: room for 6, double. 5 goes in slot 3, len 4. A bigger buffer may move to a new address, with the items copied there: the allocator decides that, while running. It is drawn in place.",
  },
  {
    line: 3,
    at: "xs.push(9)",
    lit: [core, xsChip],
    labels: { [core]: "4 < 6", [xsChip]: "xs" },
    regs: regs(["count", "1"], ["len < cap", "4 < 6: yes"], ["address", "0x2020", "ptr + 4 × 8"]),
    heap: heap(FIVE, "none"),
    output: "1",
    caption: "Now there is room: count 1 and 4 < 6, so the code the compiler wrote stores 9 at ptr + 4 × 8 and sets len to 5. The runtime isn't called at all.",
  },
  {
    line: 4,
    at: "xs[:2]",
    lit: [core, xsChip, firstsChip],
    labels: { [xsChip]: "xs", [firstsChip]: "firsts" },
    packets: [{ from: xsChip, to: firstsChip, label: "3, 1" }],
    heap: heap({ ...FIVE, items: [lit("3"), lit("1"), "4", "5", "9", ROOM] }, "live", false, [lit("3"), lit("1")]),
    output: "1",
    caption:
      "A slice is a new value: the language says so, and the runtime copies at once, while running. It makes a new list object and a buffer of exactly 2 slots, and copies 3 and 1 into them. xs is not changed: still count 1.",
  },
  {
    line: 5,
    at: "xs[0] = 7",
    lit: [core, xsChip],
    labels: { [xsChip]: "xs" },
    regs: regs(["count", "1"], ["index < len", "0 < 5: yes"], ["address", "0x2000"]),
    heap: heap(SEVEN, "live"),
    output: "1",
    caption: "Setting an item checks the same way inline: count 1, and 0 < len 5. 7 goes in place at 0x2000. firsts still holds 3: it is a copy, not a view of xs.",
  },
  {
    line: 6,
    at: "xs.swap(0, 4)",
    lit: [core, xsChip],
    labels: { [core]: "swap", [xsChip]: "xs" },
    regs: regs(["i, j", "0, 4"], ["i, j < len", "0 < 5, 4 < 5: yes"], ["count", "1", "nobody else holds it: no copy"]),
    heap: heap(SWAPPED, "live"),
    output: "1",
    caption:
      "swap is a call into the runtime, while running. It checks both indices against len 5, as xs[i] does: an index out of range would stop the program, with xs unchanged. The count is 1, so nothing else holds xs and nothing is copied. The two 8-byte words at 0x2000 and 0x2020 change places. Nothing is counted: text and records swap the same way, their words moving.",
  },
  {
    line: 7,
    at: "print(firsts)",
    lit: [core, "kernel.files"],
    labels: { "kernel.files": "output" },
    packets: [{ from: core, to: "kernel.files", label: "[3, 1]" }],
    heap: heap({ ...SWAPPED, items: ["9", "1", "4", "5", "7", ROOM] }, "freed"),
    output: "1\n[3, 1]",
    caption:
      "firsts' last use, marked by the compiler while compiling: print takes the list and lets it go. Its count falls to 0, so the runtime frees its object and buffer while running. They go grey where they were.",
  },
  {
    line: 8,
    at: "print(xs)",
    lit: [core, "kernel.files"],
    labels: { "kernel.files": "output" },
    packets: [{ from: core, to: "kernel.files", label: "[9, 1, 4, 5, 7]" }],
    heap: heap({ ...SWAPPED, count: 0, items: ["9", "1", "4", "5", "7", ROOM] }, "freed", true),
    output: "1\n[3, 1]\n[9, 1, 4, 5, 7]",
    caption: "xs's last use, also marked by the compiler: freed the same way. Nothing searched for garbage: each list was let go at its last use.",
  },
];

checkHeap("list-buffer", frames);

export const listBufferScene: Scene = {
  id: "list-buffer",
  title: "A list in memory: index, push, grow, slice, swap",
  caveman: "List is box with count. Items in other box, side by side. Read item: check, then one load. Push: room? Put there. No room? Bigger box. Slice: new list, copy. Swap: two item trade place, nothing copy.",
  sample: "explain/list-buffer.native",
  code: "main",
  board: [
    { kind: "cpu", col: 0, row: 0, label: "CPU: core 0 runs it" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the heap lives here" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  panels: ["regs", "heap"],
  heapEmpty: "Nothing yet.",
  lastUse: { 7: ["firsts"], 8: ["xs"] },
  spec: "2.2",
  frames,
  note: `Drawn for kavach run --native. Every object starts with the same 24-byte header: the count, then words saying what kind of object it is and its type. "room" is space the buffer has but doesn't use yet; "·" alone is free memory. A list written out gets room for exactly its items, as here; a list of small numbers, packed, rounds that up to whole 8-byte words: [UInt8] of 3 has room for 8, [Int32] of 3 for 4. Addresses are examples: they differ on every run.`,
};
