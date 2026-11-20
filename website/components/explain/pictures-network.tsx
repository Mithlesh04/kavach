import { HW, fit, partStyle, type Picture } from "./picture-parts";

// The network: this computer's network card, a cable out to the internet,
// and a server far away. And the operating system's kernel: not a part
// you can touch, but the layer of software every program goes through to
// reach files, the network, memory and the other programs.

/** A network card, its cable, the internet and a remote server. Parts: `nic`, `cable`, `cloud`, `server`. */
export const net: Picture = {
  name: "A network card, a cable to the internet, and a remote server",
  anchor: (part) => (part === "server" ? [200, 75] : part === "cloud" ? [146, 60] : part === "cable" ? [112, 70] : [40, 75]),
  draw: ({ on, tag }) => {
    const cable = on("cable");
    return (
      <g>
        <rect x="0" y="46" width="84" height="58" rx="3" fill={HW.pcb} stroke={HW.pcbLine} strokeWidth="1.5" />
        {Array.from({ length: 7 }, (_, i) => <rect key={i} x={8 + i * 10} y="100" width="6" height="10" rx="1" fill={HW.gold} />)}
        <rect x="20" y="58" width="30" height="30" rx="2" {...partStyle(on("nic"), HW.chip, HW.chipLine)} className={on("nic") ? "scene-lit" : undefined} />
        <text x="35" y="76" textAnchor="middle" fontSize="6.5" fill={HW.soft}>NIC</text>
        <rect x="84" y="40" width="8" height="70" rx="1" fill={HW.metal} />
        <rect x="86" y="66" width="10" height="14" rx="1" fill="#2a2d32" />
        <path d="M96 73 C112 73 112 62 124 62" fill="none" stroke={cable ? HW.lit : "#3b6fb0"} strokeWidth="3" strokeLinecap="round" />
        <path
          d="M126 70 a10 10 0 0 1 2 -19 a13 13 0 0 1 24 -4 a10 10 0 0 1 15 9 a8 8 0 0 1 -2 14 z"
          className={on("cloud") ? "fill-accent-soft stroke-accent" : "fill-chip stroke-line"}
          strokeWidth="1.5"
        />
        <text x="146" y="64" textAnchor="middle" fontSize="7" className="fill-soft">internet</text>
        <path d="M166 62 C172 62 172 75 180 75" fill="none" stroke={cable ? HW.lit : "#3b6fb0"} strokeWidth="2" strokeDasharray="4 3" />
        {[30, 62, 94].map((y, i) => (
          <g key={y}>
            <rect x="180" y={y} width="40" height="28" rx="2" {...partStyle(on("server"), "#2a2e35", "#454b55")} />
            <circle cx="187" cy={y + 8} r="2" fill={on("server") && i === 1 ? "#61c554" : "#3f6f45"} />
            <rect x="193" y={y + 17} width="22" height="3" rx="1" fill="#454b55" />
          </g>
        ))}
        {tag("server") && (
          <text x="200" y="22" textAnchor="middle" fontSize="8.5" fontWeight="700" className="fill-ink font-mono">
            {fit(tag("server") ?? "", 12)}
          </text>
        )}
        {tag("nic") && (
          <text x="42" y="36" textAnchor="middle" fontSize="8.5" fontWeight="700" className="fill-ink font-mono">
            {fit(tag("nic") ?? "", 14)}
          </text>
        )}
      </g>
    );
  },
};

/** The kernel's services, left to right. A scene lights one as `kernel.files`. */
export const KERNEL_PARTS = ["scheduler", "memory", "files", "network", "devices"] as const;

/**
 * How the kernel band is laid out in `width` units: its name on the left
 * and its services beside it, or, when narrow (a phone's one column), its
 * name above and the services in a row below.
 */
export function kernelLayout(width: number) {
  const narrow = width < 400;
  const left = narrow ? 6 : 140;
  const top = narrow ? 24 : 7;
  const h = narrow ? 58 : 40;
  return { h, left, top, slotH: narrow ? 28 : 26, slot: (width - left - (narrow ? 6 : 0)) / KERNEL_PARTS.length, narrow };
}

/** Where a kernel service sits in a band `width` wide. */
export function kernelAnchor(part: string | undefined, width: number): [number, number] {
  const k = kernelLayout(width);
  const i = KERNEL_PARTS.indexOf(part as (typeof KERNEL_PARTS)[number]);
  if (i < 0) return [width / 2, k.h / 2];
  return [k.left + k.slot * i + k.slot / 2, k.top + k.slotH / 2];
}

/** The operating system's kernel: a thin layer across the board, with its services. */
export function KernelBand({ width, on, tag }: { width: number; on: (part: string) => boolean; tag: (part: string) => string | undefined }) {
  const k = kernelLayout(width);
  return (
    <g>
      <rect x="0" y="0" width={width} height={k.h} rx="6" className={on("all") ? "fill-accent-soft stroke-accent" : "fill-chip stroke-line"} strokeWidth="1.5" />
      <text x="12" y={k.narrow ? 15 : 17} fontSize="10" fontWeight="800" className="fill-ink">OS kernel</text>
      <text x={k.narrow ? 66 : 12} y={k.narrow ? 15 : 30} fontSize="7.5" className="fill-soft">every program goes through it</text>
      {KERNEL_PARTS.map((p, i) => {
        const lit = on(p);
        return (
          <g key={p}>
            <rect x={k.left + k.slot * i + 2} y={k.top} width={k.slot - 4} height={k.slotH} rx="4" className={lit ? "fill-accent-soft stroke-accent scene-lit" : "fill-panel stroke-line"} strokeWidth={lit ? 2 : 1} />
            <text x={k.left + k.slot * i + k.slot / 2} y={k.top + k.slotH / 2 + 3} textAnchor="middle" fontSize="8.5" fontWeight={lit ? 800 : 500} className={lit ? "fill-ink" : "fill-soft"}>
              {fit(tag(p) ?? p, Math.max(4, Math.floor(k.slot / 5.3)))}
            </text>
          </g>
        );
      })}
    </g>
  );
}
