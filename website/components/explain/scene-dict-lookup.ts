import { TINT, checkHeap, freeAt, header, padded, place, type CellIn } from "./heap-cells";
import type { HeapObject, Scene, SceneFrame, Slot } from "./types";

// A dict with text keys, looked up before and after it has an index: the
// sample explain/dict-lookup.native.kav (`kavach run --native`).
// Checked in the source (2026-11-21), runtime/native/src/:
// - A dict is a 64-byte object (heap.rs:122-131) holding its table
//   (Data::Map(Box<Table>), heap.rs:166).
// - table.rs: entries in insertion order, key and value in turn (`slots`,
//   66-67); each entry's hash kept (`hashes`, 68); an index of u32, entry
//   number + 1, 0 = empty, a power of two at least twice the entries
//   (`index`, 69-73). No index up to SMALL = 8 entries when the keys are
//   not integers (31-33, push 329-341); `find` then compares the stored
//   hashes in order (164-169), else probes from hash & mask, one step at a
//   time (170-182). The 9th entry builds an index of
//   (9 * 2).next_power_of_two() = 32 positions (grow_if_full, 358-368),
//   placing every entry from its kept hash (rebuild, 384-405).
// - A text key is hashed 8 bytes at a time, mixed with a seed picked at
//   random when the program starts (rt_maps.rs text_hash 106-110;
//   hashing.rs seed 13-18, FastHasher::write 53-61). The key text is
//   compared too (rt_maps.rs get_text 117-127).
// - `m[key] ?? fallback` with a text key and plain values (numbers, Bools,
//   Chars) is one call that makes no optional (rt_maps.rs
//   kv_map_get_str_or, 131-135; cranelift maps.rs text_keys_plain, 16-29).
//   This sample's values are Int.
// - Integer keys use the `fast` index of 16-byte {key, value} buckets,
//   a cheap hash 2 * (key ^ seed), and switch to the strong hash for good
//   after a probe longer than 64 (table.rs:14-22, 39-52, 254, 320).
// The index positions are an example: a real run's seed moves them.

const core = "cpu.core0";
const chip = "ram.chip3";

const KEYS = ["asha", "ravi", "omar", "bo", "cy", "di", "ed", "fay", "gus"];
const VALUES = ["31", "27", "45", "20", "20", "20", "20", "20", "20"];
const SIZE = 32;

/** An example hash position for a key in an index of 32: stands in for the seeded hash. */
function home(key: string): number {
  let h = 7;
  for (const ch of key) h = (h * 31 + ch.charCodeAt(0)) % 9973;
  return h % SIZE;
}

/** The index after all 9 entries are placed, probing one step on: the entry number + 1, or 0. */
const INDEX: number[] = (() => {
  const index = Array<number>(SIZE).fill(0);
  KEYS.forEach((k, e) => {
    let at = home(k);
    while (index[at] !== 0) at = (at + 1) % SIZE;
    index[at] = e + 1;
  });
  return index;
})();

/** The first key that found its position taken, and where it went: the example of a probe. */
const BUMPED = KEYS.map((k, e) => ({ k, home: home(k), at: INDEX.indexOf(e + 1) })).find((x) => x.at !== x.home);
const BUMP_TEXT = BUMPED ? ` Here "${BUMPED.k}" found position ${BUMPED.home} taken, so it went on to ${BUMPED.at}.` : "";

/** Where "omar" sits in the index, and how many positions its lookup reads. */
const OMAR_AT = INDEX.indexOf(3);
const OMAR_PROBES = ((OMAR_AT - home("omar") + SIZE) % SIZE) + 1;

const DICT = 0x1000;
const ENTRIES = 0x2000;
const HASHES = 0x2100;
const INDEX_AT = 0x2200;

type View = { n: number; index?: boolean; litEntry?: number; litHashes?: number[]; litIndex?: number[] };

/** The heap with the first `n` entries: the dict object, its entries, their hashes and the index, always at their places. */
function heap(v: View): HeapObject[] {
  const entries: CellIn[] = KEYS.slice(0, v.n).map((k, e) => ({ text: `${k}: ${VALUES[e]}`, note: `entry ${e + 1}`, lit: e === v.litEntry }));
  const hashes: CellIn[] = KEYS.slice(0, v.n).map((k, e) => ({ text: `h(${k})`, lit: v.litHashes?.includes(e) }));
  return [
    place("ages: a dict object, 64 bytes (one cache line)", DICT, [...header(1), { text: "→ table", note: "its table: where the 3 buffers are" }], { tint: TINT.a }),
    place("entries, in the order added: key, then value, 16 bytes each", ENTRIES, padded(entries, KEYS.length), { step: 16, tint: TINT.a }),
    place("the hash of each entry's key, kept", HASHES, padded(hashes, KEYS.length), { tint: TINT.b }),
    v.index
      ? place("index: 32 positions, 4 bytes each (an entry's number, 0 = empty)", INDEX_AT, INDEX.map((e, i) => ({ text: String(e), lit: v.litIndex?.includes(i) })), { step: 4, tint: TINT.c })
      : freeAt(INDEX_AT, SIZE, 4),
  ];
}

const regs = (...r: [string, string, string?][]): Slot[] => r.map(([name, value, note]) => ({ name, value, note }));
const at = (line: number, text: string, extra: Partial<SceneFrame> & { caption: string; heap: HeapObject[] }): SceneFrame => ({
  line,
  at: text,
  lit: [core, chip],
  labels: { [chip]: "ages" },
  ...extra,
});

const frames: SceneFrame[] = [
  at(0, "let ages", {
    heap: heap({ n: 3, litHashes: [0, 1, 2] }),
    caption:
      "The runtime makes the dict while running: one 64-byte object with the count, pointing to its table. The entries sit in the order they were added, key then value, and each key's hash is kept beside them. A key is a pointer to its text. With up to 8 entries, and keys that aren't whole numbers, there is no index.",
  }),
  at(1, "ages[\"ravi\"]", {
    labels: { [core]: "hash \"ravi\"", [chip]: "ages" },
    regs: regs(["key", "\"ravi\""], ["hash", "h(ravi)", "the bytes, 8 at a time, mixed with the seed"]),
    heap: heap({ n: 3 }),
    caption:
      "To find \"ravi\", the core hashes its bytes, 8 at a time, mixed with this run's seed. The runtime picked the seed at random when the program started, so nobody can know ahead which keys collide.",
  }),
  at(1, "?? 0", {
    labels: { [core]: "compare hashes", [chip]: "ages" },
    packets: [{ from: chip, to: core, label: "27" }],
    regs: regs(["hash", "h(ravi)"], ["compared", "h(asha), h(ravi)", "in order, until one matches"], ["result", "27"]),
    heap: heap({ n: 3, litHashes: [0, 1], litEntry: 1 }),
    output: "27",
    caption:
      "No index, so the runtime compares the stored hashes in order: h(asha) differs, h(ravi) matches. It compares the key's text too, to be sure, then reads the value beside it: 27. With text keys and plain values, whole numbers here, a read with ?? is one runtime call that makes no optional value.",
  }),
  at(2, "ages[\"zoe\"]", {
    labels: { [core]: "no match", [chip]: "ages" },
    regs: regs(["hash", "h(zoe)"], ["compared", "3 hashes", "none matches"], ["result", "0", "from ?? 0"]),
    heap: heap({ n: 3, litHashes: [0, 1, 2] }),
    output: "27\n0",
    caption: "h(zoe) matches none of the three stored hashes, so \"zoe\" isn't there, and no key text needed comparing. The read gives none, and ?? gives 0.",
  }),
  at(4, "ages[name] = 20", {
    heap: heap({ n: 8, litHashes: [3, 4, 5, 6, 7] }),
    output: "27\n0",
    caption:
      "Five rounds add bo to fay. Nobody else holds the dict: the runtime sees count 1 while running, so each new entry goes at the end, in place, with its hash. 8 entries: still no index. When a buffer is full the runtime grows it while running, and it may move; it is drawn in place.",
  }),
  at(4, "ages[name] = 20", {
    heap: heap({ n: 9, index: true, litIndex: INDEX.flatMap((e, i) => (e ? [i] : [])) }),
    output: "27\n0",
    caption:
      `The 9th entry, "gus": past 8, the runtime builds an index while running. 32 positions, a power of two at least twice the entries, so it stays at most half full. Each entry goes in at its hash's position, or the next free one.${BUMP_TEXT} The hashes were kept, so no key is hashed again.`,
  }),
  at(6, "ages[\"omar\"]", {
    labels: { [core]: `position ${home("omar")}`, [chip]: "ages" },
    packets: [{ from: chip, to: core, label: "45" }],
    regs: regs(["hash", "h(omar)"], ["position", String(home("omar")), "h(omar) & 31"], ["probes", String(OMAR_PROBES)], ["result", "45"]),
    heap: heap({ n: 9, index: true, litIndex: [OMAR_AT], litHashes: [2], litEntry: 2 }),
    output: "27\n0\n45",
    caption: `Now a lookup goes through the index: h(omar) & 31 gives position ${home("omar")}. ${OMAR_PROBES === 1 ? "It holds 3: entry 3." : `The runtime steps on until it finds 3, entry 3: ${OMAR_PROBES} positions read.`} Its kept hash matches and its key is "omar": 45. A few probes at most, however big the dict grows.`,
  }),
];

checkHeap("dict-lookup", frames);

export const dictLookupScene: Scene = {
  id: "dict-lookup",
  title: "Finding a key in a dict, before and after it has an index",
  caveman: "Dict keep things in order. Each key keep its hash. Few things? Check hashes one by one. Many things? Index say where. Few steps. Seed new each run.",
  sample: "explain/dict-lookup.native",
  code: "main",
  board: [
    { kind: "cpu", col: 0, row: 0, label: "CPU: core 0 runs it" },
    { kind: "ram", col: 1, row: 0, label: "RAM: the dict's buffers" },
  ],
  panels: ["regs", "heap"],
  lastUse: { 6: ["ages"] },
  spec: "2.2",
  frames,
  note: "Drawn for kavach run --native, with text keys. The key texts are objects of their own, not drawn. Hashes and index positions are examples: the seed changes them on every run. Keys that are whole numbers work differently: they always have an index, of 16-byte {key, value} buckets, hashed by a cheap 2 × (key ^ seed) that keeps nearby keys nearby; a probe longer than 64 positions switches that dict to the strong hash for good. Addresses are examples.",
};
