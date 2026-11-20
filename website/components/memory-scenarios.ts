// What memory holds at each line of the samples memory/step-*.kav, as spec
// 6.1 describes it and today's native runtime does it: runtime/native/src/
// heap.rs (objects and counts), table.rs (dicts and sets). A program can't
// see counts (spec 6.1), so the samples check the values each step gives,
// and components/memory-steps.tsx checks each frame's line against them.

/** An object on the heap: a holder count, then what it keeps. */
export type Obj = {
  id: string;
  kind: "list" | "text" | "dict" | "set" | "shared";
  count: number;
  /** List items, a text's characters, a dict's entries, a set's keys, or the object a Shared holds. */
  cells: string[];
  /** A dict or set past 8 entries keeps an index beside its entries. */
  index?: boolean;
  /** Changed at this step. */
  changed?: boolean;
};

/** A name in the function: a number kept in place, or the object it holds. */
export type Name = { name: string; to?: string; value?: string; ended?: boolean };

export type Frame = {
  /** The body line this step runs, from 0, and a piece of its text. */
  line: number;
  at: string;
  names: Name[];
  objs: Obj[];
  freed?: Obj[];
  caption: string;
};

const list = (id: string, count: number, cells: string[], changed?: boolean): Obj => ({ id, kind: "list", count, cells, changed });
const text = (id: string, count: number, s: string, changed?: boolean): Obj => ({ id, kind: "text", count, cells: [...s], changed });
const dict = (id: string, count: number, cells: string[], changed?: boolean, index?: boolean): Obj => ({ id, kind: "dict", count, cells, changed, index });

const A = ["2", "3", "4", "5"];

export const SCENARIOS: Record<string, Frame[]> = {
  share: [
    { line: 0, at: "const a", names: [{ name: "a", to: "#1" }], objs: [list("#1", 1, A)], caption: "One object: a header holding the count, then the four numbers side by side. The numbers are not objects of their own." },
    { line: 1, at: "const b = a", names: [{ name: "a", to: "#1" }, { name: "b", to: "#1" }], objs: [list("#1", 2, A)], caption: "Nothing is allocated and nothing is copied. b holds the same object, and its count goes from 1 to 2." },
    { line: 2, at: "print(`a=", names: [{ name: "a", ended: true }, { name: "b", to: "#1" }], objs: [list("#1", 1, A)], caption: "This is a's last use. When it ends, a lets go of the object, and the count goes back to 1." },
    { line: 3, at: "print(`b=", names: [{ name: "a", ended: true }, { name: "b", ended: true }], objs: [], freed: [list("#1", 0, A)], caption: "b's last use ends, the count reaches 0, and the object is freed at once." },
  ],
  move: [
    { line: 0, at: "const a", names: [{ name: "a", to: "#1" }], objs: [list("#1", 1, A)], caption: "One object, held by a: count 1." },
    { line: 1, at: "const b = a", names: [{ name: "a", ended: true }, { name: "b", to: "#1" }], objs: [list("#1", 1, A)], caption: "a is never used again, so this is a move: b takes the object, the count stays 1, and the name a just ends. The compiler leaves out the count update." },
    { line: 2, at: "print(`b=", names: [{ name: "a", ended: true }, { name: "b", ended: true }], objs: [], freed: [list("#1", 0, A)], caption: "b's last use: the object is freed." },
  ],
  copy: [
    { line: 0, at: "const a", names: [{ name: "a", to: "#1" }], objs: [list("#1", 1, A)], caption: "One object, held by a." },
    { line: 1, at: "let b = a", names: [{ name: "a", to: "#1" }, { name: "b", to: "#1" }], objs: [list("#1", 2, A)], caption: "b holds the same object: count 2, nothing copied yet." },
    { line: 2, at: "b.push(6)", names: [{ name: "a", to: "#1" }, { name: "b", to: "#2" }], objs: [list("#1", 1, A), list("#2", 1, [...A, "6"], true)], caption: "b changes while a still holds the object, so now, and only now, it is copied: b gets its own object with 6 added, and a keeps the original. Each count is 1." },
    { line: 3, at: "print(`a=", names: [{ name: "a", ended: true }, { name: "b", to: "#2" }], objs: [list("#2", 1, [...A, "6"])], freed: [list("#1", 0, A)], caption: "a's last use: its object is freed." },
    { line: 4, at: "print(`b=", names: [{ name: "a", ended: true }, { name: "b", ended: true }], objs: [], freed: [list("#2", 0, [...A, "6"])], caption: "b's last use: its object is freed too." },
  ],
  loop: [
    { line: 0, at: "let items", names: [{ name: "items", to: "#1" }], objs: [list("#1", 1, [])], caption: "An empty list is one object with no items yet." },
    ...[["0"], ["0", "10"], ["0", "10", "20"]].map((cells, i) => ({
      line: 2,
      at: "items.push",
      names: [{ name: "items", to: "#1" }],
      objs: [list("#1", 1, cells, true)],
      caption: i === 0
        ? "Nobody else holds the list, so push changes it in place: no copy, only the new number is written."
        : `Round ${i + 1}: in place again. When the item buffer is full it moves to a bigger one, as a Rust Vec grows, so a push costs the same on average.`,
    })),
    { line: 4, at: "print(items)", names: [{ name: "items", ended: true }], objs: [], freed: [list("#1", 0, ["0", "10", "20"])], caption: "The last use: the list is freed." },
  ],
  numbers: [
    { line: 0, at: "const a = 7", names: [{ name: "a", value: "7" }], objs: [], caption: "A number is kept in place, in the variable itself: 8 bytes, often in a register. No object, no count." },
    { line: 1, at: "let b = a", names: [{ name: "a", value: "7" }, { name: "b", value: "7" }], objs: [], caption: "b = a copies the 8 bytes. Copying a number costs no more than pointing at it would." },
    { line: 2, at: "b += 1", names: [{ name: "a", value: "7" }, { name: "b", value: "8", }], objs: [], caption: "b changes its own 8 bytes; a keeps 7." },
    { line: 3, at: "print(`a=", names: [{ name: "a", ended: true }, { name: "b", ended: true }], objs: [], caption: "Nothing to free: numbers were never objects. Bool and Char work the same way." },
  ],
  text: [
    { line: 0, at: "const s", names: [{ name: "s", to: "#1" }], objs: [text("#1", 1, "tea")], caption: "Text is one object: the count, then its characters." },
    { line: 1, at: "let t = s", names: [{ name: "s", to: "#1" }, { name: "t", to: "#1" }], objs: [text("#1", 2, "tea")], caption: "As with a list: t holds the same object, count 2, nothing copied." },
    { line: 2, at: "t +=", names: [{ name: "s", to: "#1" }, { name: "t", to: "#2" }], objs: [text("#1", 1, "tea"), text("#2", 1, "tea and cake", true)], caption: "t changes while s holds the text, so t gets its own copy with the part added. In a loop, with no other holder, += adds only the part, in place." },
    { line: 3, at: "print(`s=", names: [{ name: "s", ended: true }, { name: "t", to: "#2" }], objs: [text("#2", 1, "tea and cake")], freed: [text("#1", 0, "tea")], caption: "s's last use: its text is freed." },
    { line: 4, at: "print(`t=", names: [{ name: "s", ended: true }, { name: "t", ended: true }], objs: [], freed: [text("#2", 0, "tea and cake")], caption: "t's last use: freed too." },
  ],
  dict: [
    { line: 0, at: "let stock", names: [{ name: "stock", to: "#1" }], objs: [dict("#1", 1, [])], caption: "A dict is one object with a count, like a list. It holds its entries in the order they were added." },
    { line: 1, at: "stock[\"tea\"]", names: [{ name: "stock", to: "#1" }], objs: [dict("#1", 1, ["tea → 3"], true)], caption: "The entry goes in place, with its key's hash kept beside it. With up to 8 entries and keys that aren't whole numbers, there is no index: a lookup compares the few stored hashes." },
    { line: 2, at: "stock[\"cake\"]", names: [{ name: "stock", to: "#1" }], objs: [dict("#1", 1, ["tea → 3", "cake → 5"], true)], caption: "Nobody else holds the dict, so it changes in place, the new entry after the first." },
    { line: 4, at: "stock[`item", names: [{ name: "stock", to: "#1" }], objs: [dict("#1", 1, ["tea → 3", "cake → 5", ...Array.from({ length: 7 }, (_, i) => `item${i} → ${i}`)], true, true)], caption: "After the 7 rounds it holds 9 entries. Past 8, the table builds an index beside them, which finds a key in a few probes however big the dict is. The kept hashes mean growing it never hashes a key again." },
    { line: 6, at: "print(stock", names: [{ name: "stock", ended: true }], objs: [], freed: [dict("#1", 0, ["tea → 3", "cake → 5", "… 7 more"], false, true)], caption: "The last use: the dict, its entries and its index are freed together." },
  ],
  "dict-copy": [
    { line: 0, at: "const prices", names: [{ name: "prices", to: "#1" }], objs: [dict("#1", 1, ["tea → 3", "cake → 5"])], caption: "One dict object, count 1." },
    { line: 1, at: "let sale = prices", names: [{ name: "prices", to: "#1" }, { name: "sale", to: "#1" }], objs: [dict("#1", 2, ["tea → 3", "cake → 5"])], caption: "Shared, as a list is: count 2, nothing allocated." },
    { line: 2, at: "sale[\"tea\"] = 2", names: [{ name: "prices", to: "#1" }, { name: "sale", to: "#2" }], objs: [dict("#1", 1, ["tea → 3", "cake → 5"]), dict("#2", 1, ["tea → 2", "cake → 5"], true)], caption: "sale changes while prices still holds the dict, so sale gets its own copy. prices keeps its entries." },
    { line: 3, at: "print(`prices=", names: [{ name: "prices", ended: true }, { name: "sale", to: "#2" }], objs: [dict("#2", 1, ["tea → 2", "cake → 5"])], freed: [dict("#1", 0, ["tea → 3", "cake → 5"])], caption: "prices's last use: its dict is freed." },
    { line: 4, at: "print(`sale=", names: [{ name: "prices", ended: true }, { name: "sale", ended: true }], objs: [], freed: [dict("#2", 0, ["tea → 2", "cake → 5"])], caption: "sale's last use: freed too." },
  ],
  set: [
    { line: 0, at: "let seen", names: [{ name: "seen", to: "#1" }], objs: [{ id: "#1", kind: "set", count: 1, cells: ["tea", "cake"] }], caption: "A set is the same table as a dict, with keys only: in the order added, each with its hash." },
    { line: 1, at: "seen.add(\"tea\")", names: [{ name: "seen", to: "#1" }], objs: [{ id: "#1", kind: "set", count: 1, cells: ["tea", "cake"] }], caption: "\"tea\" is there already, so nothing changes." },
    { line: 2, at: "seen.add(\"jam\")", names: [{ name: "seen", to: "#1" }], objs: [{ id: "#1", kind: "set", count: 1, cells: ["tea", "cake", "jam"], changed: true }], caption: "A new key goes at the end, in place, as nobody else holds the set." },
    { line: 3, at: "print(seen)", names: [{ name: "seen", ended: true }], objs: [], freed: [{ id: "#1", kind: "set", count: 0, cells: ["tea", "cake", "jam"] }], caption: "The last use: the set is freed." },
  ],
  shared: [
    { line: 0, at: "Shared(", names: [{ name: "a", to: "#1" }], objs: [{ id: "#1", kind: "shared", count: 1, cells: ["#2"] }, list("#2", 1, ["2", "3"])], caption: "A Shared is an object of its own, holding the list." },
    { line: 1, at: "const b = a", names: [{ name: "a", to: "#1" }, { name: "b", to: "#1" }], objs: [{ id: "#1", kind: "shared", count: 2, cells: ["#2"] }, list("#2", 1, ["2", "3"])], caption: "Copying a Shared copies the handle: a and b hold the one Shared, count 2. This is the one way two names see one changing value." },
    { line: 2, at: "b.update", names: [{ name: "a", to: "#1" }, { name: "b", ended: true }], objs: [{ id: "#1", kind: "shared", count: 1, cells: ["#2"] }, list("#2", 1, ["2", "3", "4"], true)], caption: "After update, the Shared holds [2, 3, 4], so a sees it too: both names hold the one Shared. It was b's last use, so b lets go: count 1." },
    { line: 3, at: "print(a.get())", names: [{ name: "a", ended: true }, { name: "b", ended: true }], objs: [], freed: [{ id: "#1", kind: "shared", count: 0, cells: ["#2"] }, list("#2", 0, ["2", "3", "4"])], caption: "a's last use: the Shared is freed, and with it the list it held." },
  ],
};
