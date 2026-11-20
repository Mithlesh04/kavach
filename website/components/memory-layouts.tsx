import { C } from "./inline-code";

// "Where each type lives": spec 6.1's table, with a picture of each, drawn
// as today's native runtime lays it out (runtime/native/src/heap.rs,
// text_items.rs, table.rs). Spec 6.1 calls this "as the compiler does it
// today": a program can't tell the difference.

const box = "inline-flex h-6 min-w-6 items-center justify-center rounded border border-line bg-bg px-1 font-mono text-[11.5px] text-ink";
const count = `${box} font-bold`;

function Obj({ children, dashed }: { children: React.ReactNode; dashed?: boolean }) {
  return <span className={`inline-flex flex-wrap items-center gap-0.5 rounded-md border-[1.5px] p-0.5 ${dashed ? "border-dashed border-line-2" : "border-accent"}`}>{children}</span>;
}

const Name = ({ n }: { n: string }) => <span className="font-mono text-[12px] font-bold text-ink">{n}</span>;
const Arrow = () => <span className="text-soft">→</span>;
const Cells = ({ xs }: { xs: string[] }) => <>{xs.map((x, i) => <span key={i} className={box}>{x}</span>)}</>;

type Row = { what: React.ReactNode; where: React.ReactNode; picture: React.ReactNode };

const ROWS: Row[] = [
  {
    what: <>Numbers, <C>Bool</C>, <C>Char</C></>,
    where: "In place: 8 bytes in the variable, or inside the list or record that holds them. No object, no count, never freed on their own. b = a copies the 8 bytes: an address to share would be 8 bytes too, so sharing would save nothing.",
    picture: <><Name n="a" /> <span className={box}>7</span> <Name n="b" /> <span className={box}>7</span></>,
  },
  {
    what: "Text",
    where: "One object: a count, then the characters. Shared by t = s, moved at the last use, copied only when changed while shared.",
    picture: <><Name n="s" /> <Arrow /> <Obj><span className={count}>count 1</span><Cells xs={["t", "e", "a"]} /></Obj></>,
  },
  {
    what: "Lists",
    where: "One object: a count and the length, then a buffer with the items side by side. Numbers sit in the buffer itself.",
    picture: <><Name n="a" /> <Arrow /> <Obj><span className={count}>count 1</span><span className={box}>len 4</span></Obj> <Arrow /> <Obj dashed><Cells xs={["2", "3", "4", "5"]} /></Obj></>,
  },
  {
    what: "Text in a list, in a built program today",
    where: "Each item is two words (16 bytes). Text up to 15 bytes is kept right in them, with no object and no count; longer text points to its own object.",
    picture: <><Obj dashed><span className={box}>&quot;tea&quot;</span><span className={box}>&quot;a longer line…&quot; →</span></Obj> <Arrow /> <Obj><span className={count}>count 1</span><span className={box}>a longer line…</span></Obj></>,
  },
  {
    what: "Dicts and sets",
    where: "One object: a count, the entries in the order added (each with its hash), and, past 8 entries, an index that finds a key in a few probes. A set keeps keys only.",
    picture: <><Name n="d" /> <Arrow /> <Obj><span className={count}>count 1</span><Cells xs={["tea → 3", "cake → 5"]} /></Obj></>,
  },
  {
    what: "Records",
    where: "One object: a count, then the fields in place. A record a function never shares may never become an object: its fields stay in place, like numbers.",
    picture: <><Name n="o" /> <Arrow /> <Obj><span className={count}>count 1</span><Cells xs={["id: 7", "qty: 2"]} /></Obj></>,
  },
  {
    what: <><C>Shared</C></>,
    where: "One object holding the value. Copying a Shared copies the handle, so both names see one changing value.",
    picture: <><Name n="a" /> <Name n="b" /> <Arrow /> <Obj><span className={count}>count 2</span><span className={box}>→</span></Obj> <Arrow /> <Obj><span className={count}>count 1</span><Cells xs={["2", "3"]} /></Obj></>,
  },
  {
    what: <><C>Bytes</C>, <C>Decimal</C>, <C>BigInt</C>, tuples, unions, optionals, functions, tasks, channels, errors</>,
    where: "One object each, with a count, behaving as lists do.",
    picture: <Obj><span className={count}>count 1</span><span className={box}>…</span></Obj>,
  },
  {
    what: <>Frozen values</>,
    where: "x.freeze() is the same object, at no cost. A value sealed with freeze(key: k) is the pair (value, key).",
    picture: <><Name n="f" /> <Arrow /> <Obj><span className={count}>count 1</span><Cells xs={["2", "3"]} /></Obj></>,
  },
];

/** Where each type lives, with a picture per row. */
export function MemoryLayouts() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[14.5px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Values</th>
            <th className="py-2 pr-4 font-bold">Where they live</th>
            <th className="py-2 font-bold">Picture</th>
          </tr>
        </thead>
        <tbody>
          {ROWS.map((r, i) => (
            <tr key={i} className="border-b border-line align-top">
              <td className="py-2.5 pr-4 font-semibold text-ink">{r.what}</td>
              <td className="py-2.5 pr-4 text-muted">{r.where}</td>
              <td className="py-2.5"><span className="flex flex-wrap items-center gap-1.5">{r.picture}</span></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
