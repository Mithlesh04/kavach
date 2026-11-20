// Memory cells as the step-throughs draw them: dark 8-byte cells, each
// with its address below and maybe a note. The heap panel and the memory
// step-through (components/memory-blocks.tsx) share this look. The cells
// sit on a dark panel in both themes, like the hardware they picture.

export type Cell = { text: string; addr?: string; tint?: string; lit?: boolean; flash?: boolean; dim?: boolean; note?: string };

/** A row of cells with a title: one object, or one run of a buffer. */
export type Run = { title: string; tint?: string; cells: Cell[]; freed?: boolean };

/** The dark panel cells sit on. */
export const CELL_PANEL = "flex flex-col gap-3 rounded-lg border border-[#2f3640] bg-[#0b0e11] p-3";

export function RunView({ run }: { run: Run }) {
  return (
    <div className={`mem-in flex min-w-0 flex-col gap-1 ${run.freed ? "opacity-50" : ""}`}>
      <span className="font-mono text-[11px]" style={{ color: run.tint ?? "#9aa0a6" }}>
        {run.title}
        {run.freed && <strong className="text-[#7fc98f]"> · freed</strong>}
      </span>
      <div className="flex flex-wrap gap-[3px]">
        {run.cells.map((c, i) => (
          <div key={`${i}-${c.text}`} className="flex flex-col items-center gap-0.5">
            <span
              className={`flex h-10 min-w-[58px] items-center justify-center rounded-[3px] border px-1.5 font-mono text-[12.5px] font-semibold ${c.flash ? "mem-flash" : ""} ${c.dim ? "line-through" : ""}`}
              style={{
                background: c.lit ? "color-mix(in srgb, #e6a23c 28%, #101418)" : "#101418",
                color: c.dim ? "#7a7f86" : "#e8e6e1",
                borderColor: c.tint ?? "#2f3640",
                boxShadow: c.lit ? "0 0 10px color-mix(in srgb, #e6a23c 55%, transparent)" : "inset 0 0 0 1px #00000060",
              }}
            >
              {c.text}
            </span>
            <span className="font-mono text-[9.5px] text-[#8f887b]">{c.addr}</span>
            {c.note && <span className="max-w-[86px] text-center text-[10.5px] leading-tight text-[#b5aea2]">{c.note}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
