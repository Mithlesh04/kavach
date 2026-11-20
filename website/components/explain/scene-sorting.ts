import { TINT, addr, checkHeap, freeAt, header, place, type CellIn } from "./heap-cells";
import type { HeapObject, Scene, SceneFrame } from "./types";

// Which sort runs, and whether it copies: the sample
// explain/sorting.native.kav (`kavach run --native`). Checked in the
// source (2026-11-21), runtime/native/src/:
// - rt_text.rs sort (44-56): `unique` first, so a list nobody else holds
//   is sorted where it is and a shared one is copied (rc.rs unique,
//   157-195). Then by item type: signed whole numbers and unsigned ones,
//   Bool and Char go to the radix sort; everything else (floats, text,
//   tuples) to Rust's stable sort_by with Kavach's order.
// - radix.rs: under SMALL = 512 items, Rust's sort_unstable (24, 31-37);
//   otherwise the sign bit is flipped so signed order is unsigned order
//   (39-40), bits the same in every number are skipped (41-44), and lsd
//   (108-131) counts every pass's 8-bit digits in one read, skips a pass
//   whose digit is the same in every number, and places the numbers into
//   a scratch buffer of the same length and back (`other`, 45). With an
//   even number of passes the result ends in the list's own buffer; with
//   an odd number it is copied back (43-47).
//   0..999 differ in their low 10 bits: 2 passes.
// - A list of short text keeps each item in 16 bytes inside its buffer
//   (text_items.rs); for the sort, lists.rs with_slots (14-45) makes
//   them text objects and puts them back.
// - The radix passes on decimal digits, step by step, are RadixDemo
//   (components/radix-demo.tsx) on the same page.

const core = "cpu.core0";
const chip = "ram.chip2";
const bigChip = "ram.chip5";

const BIG = Array.from({ length: 1000 }, (_, i) => (i * 7919) % 1000);
const stableBy = (xs: number[], digit: (x: number) => number) => [...xs].sort((a, b) => digit(a) - digit(b));
const PASS1 = stableBy(BIG, (x) => x & 255);
const PASS2 = stableBy(PASS1, (x) => (x >> 8) & 255);

type State = "none" | "live" | "freed";
type View = {
  ages?: { state: State; count?: number };
  sorted?: State;
  words?: { state: State; items?: string[] };
  big?: { state: State; items?: number[]; title?: string };
  scratch?: { state: State; items?: number[] };
  lit?: boolean;
};

const AGES = ["42", "-7", "0", "1000", "3"];
const SORTED = ["-7", "0", "3", "42", "1000"];

/** A long buffer drawn as its first three slots and a cell for the rest. */
const longCells = (xs: number[], lit = false): CellIn[] => [...xs.slice(0, 3).map((x) => ({ text: String(x), lit })), { text: "…", note: "997 more, 8 B each" }];

/** A list object at `base` and its buffer at `buf`, or free memory there while it isn't made. */
function list(title: string, base: number, buf: number, state: State, len: string, cap: string, items: CellIn[], tint: string, step = 8): HeapObject[] {
  if (state === "none") return [freeAt(base, 6), freeAt(buf, items.length, step)];
  const freed = state === "freed";
  return [
    place(`${title}: a list object`, base, [...header(freed ? 0 : 1), { text: `→ ${addr(buf)}`, note: "ptr" }, len, cap], { tint, freed }),
    place(`${title}: its items`, buf, items, { tint, freed, step }),
  ];
}

/** Every place the scene uses, in address order: objects from 0x1000, buffers from 0x2000. */
function heap(v: View): HeapObject[] {
  const [agesObj, agesBuf] = list("ages", 0x1000, 0x2000, v.ages?.state ?? "live", "len 5", "cap 5", AGES, TINT.a);
  agesObj.cells[0].text = `count ${v.ages?.state === "freed" ? 0 : (v.ages?.count ?? 1)}`;
  const [sortedObj, sortedBuf] = list("sorted", 0x1030, 0x2040, v.sorted ?? "none", "len 5", "cap 5", SORTED.map((text) => ({ text, lit: v.lit && v.sorted === "live" })), TINT.b);
  const words = v.words ?? { state: "none" };
  const [wordsObj, wordsBuf] = list("the text list", 0x1060, 0x2080, words.state, "len 3", "cap 3", (words.items ?? ["pear", "Fig", "apple"]).map((t) => ({ text: t, note: "16 B, inside" })), TINT.c, 16);
  const big = v.big ?? { state: "none" };
  const [bigObj, bigBuf] = list(big.title ?? "big", 0x1090, 0x3000, big.state, "len 1000", "cap ≥ 1000", longCells(big.items ?? BIG, v.lit && big.state === "live"), TINT.d);
  const scratch = v.scratch ?? { state: "none" };
  const scratchBuf =
    scratch.state === "none"
      ? freeAt(0x5000, 4)
      : place("the sort's scratch buffer, 1,000 slots", 0x5000, longCells(scratch.items ?? BIG, scratch.state === "live"), { tint: TINT.d, freed: scratch.state === "freed" });
  return [agesObj, sortedObj, wordsObj, bigObj, agesBuf, sortedBuf, wordsBuf, bigBuf, scratchBuf];
}

const OUT = ["[42, -7, 0, 1000, 3]", "[-7, 0, 3, 42, 1000]", '["Fig", "apple", "pear"]', "0 1 999"];
const out = (n: number) => OUT.slice(0, n).join("\n");

const DONE = { ages: { state: "freed" as const }, sorted: "freed" as const, words: { state: "freed" as const, items: ["Fig", "apple", "pear"] } };

const frames: SceneFrame[] = [
  { line: 0, at: "const ages", lit: [core, chip], labels: { [chip]: "ages" }, heap: heap({}), caption: "A list of 5 whole numbers: one list object, and a buffer with the numbers in it." },
  {
    line: 1,
    at: "ages.sort()",
    lit: [core, chip],
    labels: { [core]: "copy, then sort", [chip]: "ages, sorted" },
    heap: heap({ sorted: "live", lit: true }),
    caption:
      "ages is read again on the next line, so the compiler, while compiling, makes sort get a second hold: count 2. While running, the runtime sees count 2 and copies first: sorted gets its own list, and ages keeps its order. The compiler passed the item type, Int; the runtime picks the sort from it and the length. Under 512 items: a comparison sort, Rust's unstable one, a quicksort kind. Equal whole numbers can't be told apart, so their order doesn't matter.",
  },
  { line: 2, at: "print(ages)", lit: [core, "kernel.files"], labels: { "kernel.files": "output" }, heap: heap({ ages: { state: "freed" }, sorted: "live" }), output: out(1), caption: "ages's last use, marked by the compiler while compiling: printed, then freed. It was never changed." },
  { line: 3, at: "print(sorted)", lit: [core, "kernel.files"], labels: { "kernel.files": "output" }, heap: heap({ ages: { state: "freed" }, sorted: "freed" }), output: out(2), caption: "sorted's last use: printed, then freed." },
  {
    line: 4,
    at: ".sort()",
    lit: [core, chip],
    labels: { [core]: "stable sort", [chip]: "text list" },
    heap: heap({ ages: { state: "freed" }, sorted: "freed", words: { state: "live", items: ["Fig", "apple", "pear"] } }),
    output: out(3),
    caption:
      "Text needs comparing, so the runtime, while running, picks a stable merge sort (Rust's standard sort), by Unicode code point: capital F comes before small a. This list is held by nobody else (count 1), so it isn't copied first. Its short texts sit inside its buffer, 16 bytes each; for the sort they are made into text objects and put back. Printed, then freed.",
  },
  {
    line: 7,
    at: "big.push",
    lit: [core, bigChip],
    labels: { [bigChip]: "big" },
    heap: heap({ ...DONE, big: { state: "live" } }),
    output: out(3),
    caption: "1,000 pushes fill big with 0 to 999 in a mixed order. The buffer grows as it fills, while running; it may move, and is drawn in place.",
  },
  {
    line: 9,
    at: "big.sort()",
    lit: [core, bigChip],
    labels: { [core]: "radix sort", [bigChip]: "big = order" },
    heap: heap({ ...DONE, big: { state: "live", title: "big, now order" }, scratch: { state: "live", items: [0, 0, 0] } }),
    output: out(3),
    caption:
      "big's last use, marked by the compiler while compiling: sort takes it over (a move), so the count stays 1 and the runtime sorts it where it is, no copy. order is the same list. 1,000 whole numbers, 512 or more: a radix sort. The runtime makes a scratch buffer of 1,000 slots for it, while running.",
  },
  {
    line: 9,
    at: "big.sort()",
    lit: [core, bigChip],
    labels: { [core]: "pass 1: low 8 bits", [bigChip]: "→ scratch" },
    packets: [{ from: bigChip, to: core, label: "digits" }],
    heap: heap({ ...DONE, big: { state: "live", title: "big, now order" }, scratch: { state: "live", items: PASS1 } }),
    output: out(3),
    caption:
      "One read counts every 8-bit digit of every number. The sign bit is flipped first, so negative numbers would come first; then the 54 high bits are the same in every number, and are skipped: 0 to 999 need only 10 bits, so 2 passes. Pass 1 places each number into the scratch buffer by its low 8 bits, keeping the order of numbers with the same digit. No two numbers are ever compared.",
  },
  {
    line: 9,
    at: "big.sort()",
    lit: [core, bigChip],
    labels: { [core]: "pass 2: bits 8–9", [bigChip]: "→ big" },
    heap: heap({ ...DONE, big: { state: "live", title: "big, now order", items: PASS2 }, scratch: { state: "freed", items: PASS1 } }),
    output: out(3),
    caption:
      "Pass 2 places them back into big's own buffer by bits 8 and 9: sorted. Two passes end where they began, so nothing is copied back; with an odd number of passes, the result would be in the scratch buffer and the runtime would copy it back. The runtime frees the scratch buffer.",
  },
  {
    line: 10,
    at: "order[999]",
    lit: [core, "kernel.files"],
    labels: { "kernel.files": "output" },
    heap: heap({ ...DONE, big: { state: "live", title: "big, now order", items: PASS2 }, scratch: { state: "freed", items: PASS1 } }),
    output: out(4),
    caption: "order[0] is 0, order[1] is 1, order[999] is 999: three loads, each after its index check.",
  },
];

checkHeap("sorting", frames);

export const sortingScene: Scene = {
  id: "sorting",
  title: "Which sort runs, and when sorting copies",
  caveman: "Whole numbers? Sort by digits, no compare. Few numbers? Compare. Text? Compare, keep equal ones in order. Nobody else hold list? Sort it where it is. Someone else hold it? Copy first.",
  sample: "explain/sorting.native",
  code: "main",
  board: [
    { kind: "cpu", col: 0, row: 0, label: "CPU: core 0 sorts" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the lists" },
    { kind: "kernel", col: 0, row: 1 },
  ],
  panels: ["heap"],
  lastUse: { 2: ["ages"], 3: ["sorted"], 9: ["big"], 10: ["order"] },
  spec: "14.2",
  frames,
  note: "Drawn for kavach run --native; a kavach build picks the same sorts. kavach run, the interpreter, uses the stable sort for everything, with the same results. 1,000-item buffers are drawn as their first three numbers. order is held by a local of main, so it is not drawn freed: native main leaves what its locals hold to the operating system at exit, and the interpreter frees everything when main ends. Addresses are examples.",
};
