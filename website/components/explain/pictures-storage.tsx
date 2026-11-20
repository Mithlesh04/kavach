import { HW, fit, partStyle, type Picture } from "./picture-parts";

// Where data is kept: a RAM stick (fast, empty when the power goes), an
// SSD (flash chips that keep data without power) and a hard disk (a
// spinning platter and a moving head).

const chipX = (i: number) => 14 + i * 47.5;

/**
 * A RAM stick, 400 × 72 units: its board, eight memory chips and the gold
 * contacts that plug into the computer. `lit` says which chips glow.
 */
export function RamStickShape({ lit, pulse }: { lit: (chip: number) => boolean; pulse?: boolean }) {
  return (
    <g>
      <rect x="2" y="6" width="396" height="66" rx="5" fill={HW.pcb} stroke={HW.pcbLine} strokeWidth="2" />
      <path d="M2 30h6M392 30h6" stroke={HW.pcbLine} strokeWidth="6" />
      {Array.from({ length: 8 }, (_, i) => (
        <g key={i}>
          <rect x={chipX(i)} y="16" width="38" height="34" rx="2" {...partStyle(lit(i), HW.chip, HW.chipLine)} strokeWidth={lit(i) ? 2.5 : 1} className={lit(i) && pulse ? "mem-chip" : undefined} />
          <circle cx={chipX(i) + 5} cy="21" r="1.4" fill="#555" />
        </g>
      ))}
      {Array.from({ length: 38 }, (_, i) => (i === 18 ? null : <rect key={i} x={10 + i * 10} y="58" width="6" height="14" rx="1" fill={HW.gold} />))}
    </g>
  );
}

/** A RAM stick on its own, with one chip lit and lines to its close-up below. */
export function RamStick({ chip = 2 }: { chip?: number }) {
  const x = chipX(chip);
  return (
    <svg viewBox="0 0 400 92" className="h-auto w-full max-w-[460px]" role="img" aria-label="A memory stick; one chip is shown close up below">
      <RamStickShape lit={(i) => i === chip} pulse />
      <path d={`M${x} 50 L60 92 M${x + 38} 50 L340 92`} stroke={HW.lit} strokeWidth="1.2" strokeDasharray="3 3" opacity="0.8" />
    </svg>
  );
}

const RAM_SCALE = 0.55;
const RAM_TOP = 34;

/** RAM in a scene. Parts: `chip0`…`chip7`; a chip's word shows below the stick. */
export const ram: Picture = {
  name: "A RAM stick with eight memory chips",
  anchor: (part) => {
    const m = part?.match(/^chip(\d)$/);
    const i = m ? Number(m[1]) : 3.5;
    return [(chipX(i) + 19) * RAM_SCALE, RAM_TOP + 33 * RAM_SCALE];
  },
  draw: ({ on, tag }) => (
    <g>
      <g transform={`translate(0 ${RAM_TOP}) scale(${RAM_SCALE})`}>
        <RamStickShape lit={(i) => on(`chip${i}`) || on("all")} />
      </g>
      {Array.from({ length: 8 }, (_, i) => {
        const word = tag(`chip${i}`);
        if (!word) return null;
        const x = (chipX(i) + 19) * RAM_SCALE;
        const y = RAM_TOP + 72 * RAM_SCALE + 12 + (i % 2) * 11;
        return (
          <g key={i}>
            <line x1={x} y1={RAM_TOP + 50 * RAM_SCALE} x2={x} y2={y - 8} stroke={HW.lit} strokeWidth="0.8" strokeDasharray="2 2" />
            <text x={x} y={y} textAnchor="middle" fontSize="8.5" fontWeight="700" className="fill-ink font-mono">
              {fit(word, 12)}
            </text>
          </g>
        );
      })}
    </g>
  ),
};

/** An M.2 SSD: gold fingers, a controller, a small cache and two flash (NAND) chips. Parts: `controller`, `nand0`, `nand1`. */
export const ssd: Picture = {
  name: "An SSD: a controller and flash chips on a small board",
  anchor: (part) => (part === "controller" ? [47, 75] : part === "nand0" ? [127, 75] : part === "nand1" ? [183, 75] : [120, 75]),
  draw: ({ on, tag }) => (
    <g>
      <rect x="0" y="46" width="216" height="58" rx="4" fill={HW.pcb} stroke={HW.pcbLine} strokeWidth="2" />
      <path d="M216 68a7 7 0 0 0 0 14" fill="#cfd3d8" stroke={HW.pcbLine} />
      {Array.from({ length: 9 }, (_, i) => (i === 6 ? null : <rect key={i} x="1" y={50 + i * 6} width="9" height="4" rx="1" fill={HW.gold} />))}
      <rect x="30" y="58" width="34" height="34" rx="2" {...partStyle(on("controller"), "#1b1b1b", HW.chipLine)} />
      <text x="47" y="78" textAnchor="middle" fontSize="6.5" fill={HW.soft}>ctrl</text>
      <rect x="72" y="63" width="20" height="24" rx="2" fill={HW.chip} stroke={HW.chipLine} />
      {[0, 1].map((i) => (
        <g key={i}>
          <rect x={102 + i * 56} y="55" width="50" height="40" rx="2" {...partStyle(on(`nand${i}`), HW.chip, HW.chipLine)} className={on(`nand${i}`) ? "scene-lit" : undefined} />
          <text x={127 + i * 56} y="78" textAnchor="middle" fontSize="7.5" fill={HW.soft}>
            {fit(tag(`nand${i}`) ?? "flash", 10)}
          </text>
        </g>
      ))}
      {tag("controller") && (
        <text x="47" y="40" textAnchor="middle" fontSize="8.5" fontWeight="700" className="fill-ink font-mono">
          {fit(tag("controller") ?? "", 16)}
        </text>
      )}
    </g>
  ),
};

/** A hard disk with its lid off: the platter, and the arm with its head. Parts: `platter`, `head`. */
export const hdd: Picture = {
  name: "A hard disk: a spinning platter and a moving head",
  anchor: (part) => (part === "head" ? [128, 50] : [100, 72]),
  draw: ({ on, tag }) => {
    const platter = on("platter");
    const head = on("head");
    return (
      <g>
        <rect x="28" y="0" width="164" height="150" rx="8" fill="#5d636b" stroke="#41464d" strokeWidth="2" />
        <rect x="34" y="6" width="152" height="138" rx="5" fill="#7d838b" />
        <circle cx="100" cy="72" r="56" fill="#c3c8cf" stroke={platter ? HW.lit : "#9aa1aa"} strokeWidth={platter ? 3 : 1.5} className={platter ? "scene-lit" : undefined} />
        <circle cx="100" cy="72" r="38" fill="none" stroke="#adb3bb" strokeWidth="0.8" />
        <circle cx="100" cy="72" r="11" fill="#8d939b" stroke="#6b7179" />
        <circle cx="166" cy="122" r="10" fill="#4b5058" />
        <path d="M166 122 L131 52" stroke={head ? HW.lit : "#3a3e44"} strokeWidth="6" strokeLinecap="round" />
        <rect x="124" y="44" width="10" height="10" rx="2" fill={head ? HW.lit : "#2a2d32"} />
        {(tag("platter") || tag("head")) && (
          <text x="110" y="140" textAnchor="middle" fontSize="8.5" fontWeight="700" fill={HW.ink} className="font-mono">
            {fit(tag("head") ?? tag("platter") ?? "", 24)}
          </text>
        )}
      </g>
    );
  },
};
