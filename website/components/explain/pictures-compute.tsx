import { HW, fit, partStyle, type Picture } from "./picture-parts";

// The CPU and the GPU, as their chips look from above. A CPU has a few
// big cores, each with its own registers and L1 cache, and an L3 cache
// they share. A GPU has many small cores in groups, and memory of its own
// (VRAM) beside them.

/** Where each core sits on the CPU's die: 1 to 8 cores, two rows at most. */
function coreBoxes(n: number) {
  const cols = n <= 2 ? n : n <= 4 ? 2 : 4;
  const rows = Math.ceil(n / cols);
  const gap = 6;
  const w = (156 - gap * (cols - 1)) / cols;
  const h = (82 - gap * (rows - 1)) / rows;
  return Array.from({ length: n }, (_, i) => ({ x: 32 + (i % cols) * (w + gap), y: 20 + Math.floor(i / cols) * (h + gap), w, h }));
}

/**
 * A CPU package: gold contacts on its edges, the die, the cores and the
 * shared L3 cache. Parts: `core0`…`core7`, `l3`.
 */
export const cpu: Picture = {
  name: "A CPU with its cores and shared cache",
  anchor: (part, cores) => {
    const m = part?.match(/^core(\d)$/);
    if (m) {
      const b = coreBoxes(cores)[Number(m[1])] ?? coreBoxes(cores)[0];
      return [b.x + b.w / 2, b.y + b.h / 2];
    }
    if (part === "l3") return [110, 119];
    return [110, 75];
  },
  draw: ({ on, tag, cores = 4 }) => (
    <g>
      <rect x="10" y="0" width="200" height="150" rx="7" fill="#2c3b30" stroke="#18231b" strokeWidth="2" />
      {Array.from({ length: 12 }, (_, i) => (
        <g key={i} fill={HW.gold}>
          <rect x="11" y={8 + i * 11.5} width="5" height="5" rx="1" />
          <rect x="204" y={8 + i * 11.5} width="5" height="5" rx="1" />
        </g>
      ))}
      <rect x="24" y="10" width="172" height="130" rx="4" fill={HW.die} stroke="#3a3f47" />
      {coreBoxes(cores).map((b, i) => {
        const lit = on(`core${i}`);
        const word = tag(`core${i}`);
        const big = b.w > 50;
        return (
          <g key={i}>
            <rect x={b.x} y={b.y} width={b.w} height={b.h} rx="3" {...partStyle(lit, "#2b3038", "#3f4651")} className={lit ? "scene-lit" : undefined} />
            {/* registers: a row of small cells at the core's top */}
            {Array.from({ length: big ? 6 : 3 }, (_, r) => (
              <rect key={r} x={b.x + 4 + r * 7} y={b.y + 4} width="5" height="5" rx="0.8" fill={lit ? HW.lit : "#5b6470"} />
            ))}
            <text x={b.x + b.w - 4} y={b.y + 9.5} textAnchor="end" fontSize="7.5" fill={HW.soft}>
              {big ? `core ${i}` : i}
            </text>
            {/* the L1 cache: a strip along the core's bottom */}
            <rect x={b.x + 4} y={b.y + b.h - 9} width={b.w - 8} height="5" rx="1" fill={lit ? "#6b4a14" : "#3a414c"} />
            {big && b.h > 30 && (
              <text x={b.x + b.w - 5} y={b.y + b.h - 11.5} textAnchor="end" fontSize="6.5" fill={HW.soft}>
                L1
              </text>
            )}
            {word && (
              <text x={b.x + b.w / 2} y={b.y + b.h / 2 + 3} textAnchor="middle" fontSize={big ? 9.5 : 7.5} fontWeight="700" fill={HW.ink} className="font-mono">
                {fit(word, Math.floor(b.w / 6))}
              </text>
            )}
          </g>
        );
      })}
      <rect x="32" y="108" width="156" height="22" rx="3" {...partStyle(on("l3"), "#262a31", "#3f4651")} />
      <text x="110" y="122" textAnchor="middle" fontSize="8.5" fill={HW.soft}>
        {tag("l3") ?? "L3 cache, shared by the cores"}
      </text>
    </g>
  ),
};

/** Where each group of small cores sits on the GPU's die. */
const GROUPS = [0, 1, 2, 3].map((i) => ({ x: 62 + (i % 2) * 50, y: 30 + Math.floor(i / 2) * 46 }));

/**
 * A graphics card: a black board, the GPU die with four groups of 16
 * small cores, and VRAM chips around it. Parts: `group0`…`group3`,
 * `die`, `vram`.
 */
export const gpu: Picture = {
  name: "A GPU: many small cores in groups, and its own memory (VRAM)",
  anchor: (part) => {
    const m = part?.match(/^group(\d)$/);
    if (m) {
      const g = GROUPS[Number(m[1])] ?? GROUPS[0];
      return [g.x + 21, g.y + 19];
    }
    if (part === "vram") return [190, 75];
    return [104, 75];
  },
  draw: ({ on, tag }) => {
    const vram = on("vram");
    return (
      <g>
        <rect x="0" y="6" width="220" height="138" rx="6" fill="#16181c" stroke="#2a2e35" strokeWidth="2" />
        {Array.from({ length: 22 }, (_, i) => (
          <rect key={i} x={12 + i * 9} y="138" width="6" height="10" rx="1" fill={HW.gold} />
        ))}
        <rect x="54" y="22" width="108" height="104" rx="4" fill={HW.die} stroke={on("die") ? HW.lit : "#3a3f47"} strokeWidth={on("die") ? 2.2 : 1} />
        {GROUPS.map((g, i) => {
          const lit = on(`group${i}`) || on("die");
          return (
            <g key={i}>
              <rect x={g.x - 3} y={g.y - 3} width="48" height="44" rx="2" fill="none" stroke={lit ? HW.lit : "#3f4651"} strokeWidth={lit ? 1.6 : 0.8} />
              {Array.from({ length: 16 }, (_, c) => (
                <rect key={c} x={g.x + (c % 4) * 11} y={g.y + Math.floor(c / 4) * 9.5} width="8" height="7" rx="1" fill={lit ? HW.lit : "#3b424d"} className={lit ? "scene-lit" : undefined} />
              ))}
            </g>
          );
        })}
        {[24, 60, 96].map((y) => (
          <g key={y}>
            <rect x="12" y={y} width="30" height="24" rx="2" {...partStyle(vram, HW.chip, HW.chipLine)} />
            <rect x="176" y={y} width="30" height="24" rx="2" {...partStyle(vram, HW.chip, HW.chipLine)} />
          </g>
        ))}
        <text x="27" y="131" textAnchor="middle" fontSize="8" fill={HW.soft}>VRAM</text>
        <text x="191" y="131" textAnchor="middle" fontSize="8" fill={HW.soft}>VRAM</text>
        {(tag("vram") || tag("die")) && (
          <text x="108" y="18" textAnchor="middle" fontSize="9" fontWeight="700" fill={HW.ink} className="font-mono">
            {fit(tag("die") ?? tag("vram") ?? "", 30)}
          </text>
        )}
      </g>
    );
  },
};
