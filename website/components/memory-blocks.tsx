"use client";

import { SCENARIOS, type Frame, type Obj } from "./memory-scenarios";
import { StepControls, useSteps } from "./step-player";

// One picture of memory per step, drawn as memory looks: a RAM stick, one
// chip lit, and that chip's cells below it, 8 bytes each. The stack's cells
// hold the function's names; the heap's hold the objects, a count first.
// The data is in memory-scenarios.ts; memory-steps.tsx checks it against a
// sample. Addresses are examples: they differ on every run.

const COLORS: Record<string, string> = { "#1": "#e6a23c", "#2": "#5fbf7f", "#3": "#c586c0" };

type Cell = { text: string; addr?: string; tint?: string; lit?: boolean; flash?: boolean; dim?: boolean; note?: string };
type Run = { title: string; tint?: string; cells: Cell[]; freed?: boolean };

const hex = (n: number) => `0x${n.toString(16).padStart(4, "0")}`;
const base = (id: string) => 0x1000 + (Number(id.slice(1)) - 1) * 0x100;
const STACK = 0x7ffde000;

/** Text as the words it fills inside its object: 8 bytes a word. */
const words = (s: string) => (s.match(/[\s\S]{1,8}/g) ?? [""]).map((w) => w.replace(/ /g, "␣"));

/** The cells of one object, and of its item buffer or entries when it has one. */
function objRuns(o: Obj, before: number | undefined, freed: boolean): Run[] {
  const tint = COLORS[o.id];
  const at = base(o.id);
  const lit = !freed && o.changed;
  const head: Cell = { text: freed ? "count 0" : `count ${o.count}`, note: "header, 24 B", flash: before !== undefined && before !== o.count };
  const title = `${o.id} ${o.kind === "shared" ? "Shared" : o.kind}`;
  const cells = (xs: Cell[]) => xs.map((c, i) => ({ ...c, addr: hex(at + 8 * (i === 0 ? 0 : i + 2)), tint, dim: freed }));
  if (o.kind === "list") {
    const buf = at + 0x60;
    return [
      { title, tint, freed, cells: cells([head, { text: `→ ${hex(buf)}`, note: "items at" }, { text: `len ${o.cells.length}` }]) },
      {
        title: `${o.id} items`,
        tint,
        freed,
        cells: (o.cells.length ? o.cells : ["·"]).map((x, i) => ({ text: x, addr: hex(buf + 8 * i), tint, lit, dim: freed || !o.cells.length })),
      },
    ];
  }
  if (o.kind === "text") {
    const s = o.cells.join("");
    return [{ title, tint, freed, cells: cells([head, { text: `len ${s.length}` }, ...words(s).map((w) => ({ text: w, lit }))]) }];
  }
  if (o.kind === "shared") return [{ title, tint, freed, cells: cells([head, { text: `→ ${hex(base(o.cells[0]))}`, note: "holds" }]) }];
  // A dict or set: its entries in the order added, and past 8 an index.
  const entries = at + 0x60;
  return [
    { title, tint, freed, cells: cells([head, { text: `→ ${hex(entries)}`, note: "entries at" }, { text: o.index ? "index ▦" : "no index", lit: lit && o.index }]) },
    {
      title: `${o.id} entries, in order`,
      tint,
      freed,
      cells: (o.cells.length ? o.cells : ["·"]).map((x, i) => ({ text: x, addr: hex(entries + 16 * i), tint, lit, dim: freed || !o.cells.length })),
    },
  ];
}

function runsOf(f: Frame, prev?: Frame): Run[] {
  const stack: Run = {
    title: "stack: the function's names",
    cells: f.names.map((n, i) => ({
      text: n.ended ? "—" : n.value ?? `→ ${hex(base(n.to!))}`,
      addr: hex(STACK + 8 * i).replace("0x7ffd", "0x7ffd_"),
      note: n.ended ? `${n.name} ended` : n.name,
      tint: n.to && !n.ended ? COLORS[n.to] : undefined,
      dim: n.ended,
      flash: n.value !== undefined && prev?.names.find((p) => p.name === n.name)?.value !== n.value,
    })),
  };
  const before = (id: string) => prev?.objs.find((o) => o.id === id)?.count;
  return [stack, ...f.objs.flatMap((o) => objRuns(o, before(o.id), false)), ...(f.freed ?? []).flatMap((o) => objRuns(o, undefined, true))];
}

function RamStick() {
  const chips = Array.from({ length: 8 }, (_, i) => i);
  return (
    <svg viewBox="0 0 400 92" className="h-auto w-full max-w-[460px]" role="img" aria-label="A memory stick; one chip is shown close up below">
      <rect x="2" y="6" width="396" height="66" rx="5" fill="#1f5a3a" stroke="#123824" strokeWidth="2" />
      <path d="M2 30h6M392 30h6" stroke="#123824" strokeWidth="6" />
      {chips.map((i) => (
        <g key={i}>
          <rect x={14 + i * 47.5} y="16" width="38" height="34" rx="2" fill={i === 2 ? "#3a2a10" : "#141414"} stroke={i === 2 ? "#e6a23c" : "#2b2b2b"} strokeWidth={i === 2 ? 2.5 : 1} className={i === 2 ? "mem-chip" : undefined} />
          <circle cx={19 + i * 47.5} cy="21" r="1.4" fill="#555" />
        </g>
      ))}
      {Array.from({ length: 38 }, (_, i) => (i === 18 ? null : <rect key={i} x={10 + i * 10} y="58" width="6" height="14" rx="1" fill="#d4a93c" />))}
      <path d="M109 50 L60 92 M147 50 L340 92" stroke="#e6a23c" strokeWidth="1.2" strokeDasharray="3 3" opacity="0.8" />
    </svg>
  );
}

function RunView({ run }: { run: Run }) {
  return (
    <div className={`mem-in flex flex-col gap-1 ${run.freed ? "opacity-50" : ""}`}>
      <span className="font-mono text-[11px]" style={{ color: run.tint ?? "var(--soft)" }}>
        {run.title}
        {run.freed && <strong className="text-ok"> · freed</strong>}
      </span>
      <div className="flex flex-wrap gap-[3px]">
        {run.cells.map((c, i) => (
          <div key={`${i}-${c.text}`} className="flex flex-col items-center gap-0.5">
            <span
              className={`flex h-10 min-w-[58px] items-center justify-center rounded-[3px] border px-1.5 font-mono text-[12.5px] font-semibold ${c.flash ? "mem-flash" : ""} ${c.dim ? "line-through" : ""}`}
              style={{
                background: c.lit ? "color-mix(in srgb, var(--accent) 28%, #101418)" : "#101418",
                color: c.dim ? "#7a7f86" : "#e8e6e1",
                borderColor: c.tint ?? "#2f3640",
                boxShadow: c.lit ? "0 0 10px color-mix(in srgb, var(--accent) 55%, transparent)" : "inset 0 0 0 1px #00000060",
              }}
            >
              {c.text}
            </span>
            <span className="font-mono text-[9.5px] text-soft">{c.addr}</span>
            {c.note && <span className="max-w-[86px] text-center text-[10.5px] leading-tight text-muted">{c.note}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}

/** Steps through `memory/step-<name>.kav`: the line, and memory's cells at it. */
export function MemoryBlocks({ name, lines, output }: { name: string; lines: string[]; output: string }) {
  const frames = SCENARIOS[name];
  const steps = useSteps(frames.length, 2200);
  const f = frames[steps.frame];
  const runs = runsOf(f, frames[steps.frame - 1]);
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-panel px-4 py-4 sm:px-5" role="group" aria-label="Memory, step by step">
      <div className="grid grid-cols-[minmax(0,1fr)] gap-4 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
        <div className="flex min-w-0 flex-col gap-2">
          <pre className="m-0 overflow-x-auto rounded-lg bg-code-bg px-3 py-2.5 font-mono text-[13px] leading-relaxed text-code-ink">
            {lines.map((l, i) => (
              <div key={i} className={i === f.line ? "-mx-3 border-l-[3px] border-accent bg-accent-soft px-2.5 font-bold text-ink" : "text-code-soft"}>
                {l || " "}
              </div>
            ))}
          </pre>
          <div className="flex flex-col gap-0.5">
            <span className="text-[11.5px] text-soft">The program prints, checked:</span>
            <pre className="m-0 overflow-x-auto rounded-lg border border-line bg-bg px-3 py-1.5 font-mono text-[12.5px] text-ink">{output}</pre>
          </div>
        </div>
        <div className="flex min-w-0 flex-col gap-2">
          <RamStick />
          <div className="flex flex-col gap-3 rounded-lg border border-[#2f3640] bg-[#0b0e11] p-3">
            {runs.map((r, i) => <RunView key={`${r.title}-${i}`} run={r} />)}
            {f.objs.length === 0 && !f.freed && <span className="text-[12.5px] text-[#9aa0a6]">heap: no objects here</span>}
          </div>
          <span className="text-[11px] text-soft">Each cell is 8 bytes. Addresses are examples: they differ on every run.</span>
        </div>
      </div>
      <StepControls steps={steps} caption={f.caption} />
    </div>
  );
}
