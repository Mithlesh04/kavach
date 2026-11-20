"use client";

import { Packet, linkPath } from "./packet";
import { PIC } from "./picture-parts";
import { cpu, gpu } from "./pictures-compute";
import { keyboard, screen } from "./pictures-io";
import { KernelBand, kernelAnchor, kernelLayout, net } from "./pictures-network";
import { hdd, ram, ssd } from "./pictures-storage";
import type { Placed, Scene, SceneFrame, Spot } from "./types";

// The machine a scene runs on: its resources in a grid, the OS kernel as a
// thin band under them, the links data travels along, and this step's
// packets moving on them. One SVG, so it scales with the page's width. A
// board more than one resource wide is drawn a second time in one column,
// and phones see that one, so no picture gets too small to read.

const PICTURES = { cpu, gpu, ram, ssd, hdd, net, screen, keyboard };
const NAMES = { cpu: "CPU", gpu: "GPU", ram: "RAM", ssd: "SSD", hdd: "Hard disk", net: "Network", screen: "Screen", keyboard: "Keyboard" };

/** Each resource's cell: its picture, and room for its name below. */
const CELL = { w: PIC.w + 20, h: PIC.h + 30 };

type Hardware = Placed & { kind: keyof typeof PICTURES };

function layout(board: Placed[]) {
  const parts = board.filter((p): p is Hardware => p.kind !== "kernel");
  const cols = Math.max(1, ...parts.map((p) => p.col + 1));
  const rows = Math.max(0, ...parts.map((p) => p.row + 1));
  const kernel = board.some((p) => p.kind === "kernel");
  const w = cols * CELL.w;
  const kernelY = rows * CELL.h + 6;
  const h = kernelY + (kernel ? kernelLayout(w - 20).h + 8 : 0);
  /** Where a spot is on the board. */
  const at = (s: Spot): [number, number] => {
    const [kind, part] = s.split(".");
    if (kind === "kernel") {
      const [x, y] = kernelAnchor(part, w - 20);
      return [x + 10, y + kernelY];
    }
    const p = parts.find((q) => q.kind === kind);
    if (!p) throw new Error(`Board: the scene has no ${kind} for the spot ${s}`);
    const [x, y] = PICTURES[p.kind].anchor(part, p.cores ?? 4);
    return [p.col * CELL.w + 10 + x, p.row * CELL.h + 4 + y];
  };
  return { parts, cols, kernel, w, h, kernelY, at };
}

/** The same board in one column, top to bottom in reading order. */
const oneColumn = (board: Placed[]): Placed[] => [
  ...board
    .filter((p) => p.kind !== "kernel")
    .sort((a, b) => a.row - b.row || a.col - b.col)
    .map((p, i) => ({ ...p, col: 0, row: i })),
  ...board.filter((p) => p.kind === "kernel"),
];

/** Every link some step of the scene uses, once each. */
function links(scene: Scene) {
  const seen = new Map<string, [Spot, Spot]>();
  for (const f of scene.frames) for (const p of f.packets ?? []) seen.set([p.from, p.to].sort().join(" "), [p.from, p.to]);
  return [...seen.values()];
}

/** Whether a spot is lit, by its kind and part: `cpu.core0`, or `ram` for all of it. */
const litIn = (f: SceneFrame) => (kind: string) => (part: string) =>
  !!f.lit?.some((s) => s === `${kind}.${part}` || (s === kind && part === "all"));

function BoardSvg({ scene, board, frame, step, className }: { scene: Scene; board: Placed[]; frame: SceneFrame; step: number; className: string }) {
  const { parts, kernel, w, h, kernelY, at } = layout(board);
  const lit = litIn(frame);
  const tag = (kind: string) => (part: string) => frame.labels?.[`${kind}.${part}`];
  const now = new Set((frame.packets ?? []).map((p) => [p.from, p.to].sort().join(" ")));
  const names = board.map((p) => p.label ?? (p.kind === "kernel" ? "OS kernel" : NAMES[p.kind])).join(", ");
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className={`h-auto w-full ${className}`} style={{ maxWidth: `${Math.round(w * 1.15)}px` }} role="img" aria-label={`The machine: ${names}`}>
      {parts.map((p) => {
        const whole = !!frame.lit?.includes(p.kind);
        return (
          <g key={p.kind} transform={`translate(${p.col * CELL.w + 10} ${p.row * CELL.h + 4})`}>
            {whole && <rect x="-6" y="-4" width={PIC.w + 12} height={PIC.h + 8} rx="10" fill="none" className="stroke-accent" strokeWidth="2" strokeDasharray="5 4" />}
            {PICTURES[p.kind].draw({ on: lit(p.kind), tag: tag(p.kind), cores: p.cores })}
            <text x={PIC.w / 2} y={PIC.h + 19} textAnchor="middle" fontSize="11.5" fontWeight="800" className="fill-ink">
              {p.label ?? NAMES[p.kind]}
            </text>
          </g>
        );
      })}
      {kernel && (
        <g transform={`translate(10 ${kernelY})`}>
          <KernelBand width={w - 20} on={lit("kernel")} tag={tag("kernel")} />
        </g>
      )}
      {links(scene).map(([a, b]) => {
        const busy = now.has([a, b].sort().join(" "));
        return (
          <path
            key={`${a} ${b}`}
            d={linkPath(at(a), at(b))}
            fill="none"
            className={busy ? "stroke-accent" : "stroke-soft"}
            strokeWidth={busy ? 2 : 1}
            strokeDasharray="4 4"
            opacity={busy ? 0.9 : 0.35}
          />
        );
      })}
      {(frame.packets ?? []).map((p, i) => (
        <Packet key={`${step}-${i}`} from={at(p.from)} to={at(p.to)} label={p.label} delay={i * 350} />
      ))}
    </svg>
  );
}

/** The scene's machine at one step: wider screens see its grid, phones the same resources in one column. */
export function Board({ scene, frame, step }: { scene: Scene; frame: SceneFrame; step: number }) {
  if (layout(scene.board).cols <= 1) return <BoardSvg scene={scene} board={scene.board} frame={frame} step={step} className="" />;
  return (
    <>
      <BoardSvg scene={scene} board={scene.board} frame={frame} step={step} className="hidden sm:block" />
      <BoardSvg scene={scene} board={oneColumn(scene.board)} frame={frame} step={step} className="mx-auto max-w-[300px] sm:hidden" />
    </>
  );
}
