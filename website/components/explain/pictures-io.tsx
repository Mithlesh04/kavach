import { HW, fit, partStyle, type Picture } from "./picture-parts";

// What a person sees and types with: a screen with a terminal window, and
// a keyboard. Output ends on the screen; input starts at the keyboard.

/** A monitor showing a terminal. Parts: `text` (the terminal's last line). A tag is the line shown. */
export const screen: Picture = {
  name: "A screen showing a terminal window",
  anchor: (part) => (part === "text" ? [110, 72] : [110, 60]),
  draw: ({ on, tag }) => {
    const lit = on("text");
    const line = tag("text");
    return (
      <g>
        <rect x="14" y="4" width="192" height="116" rx="6" fill="#202328" stroke="#3a3f47" strokeWidth="2" />
        <rect x="22" y="12" width="176" height="100" rx="3" style={{ fill: "var(--term-bg)" }} stroke={lit ? HW.lit : "#2b2721"} strokeWidth={lit ? 2 : 1} />
        <rect x="22" y="12" width="176" height="12" rx="3" style={{ fill: "var(--term-bar)" }} />
        {["red", "yellow", "green"].map((c, i) => <circle key={c} cx={30 + i * 8} cy="18" r="2.4" style={{ fill: `var(--term-${c})` }} />)}
        <text x="30" y="40" fontSize="9" style={{ fill: "var(--code-soft)" }} className="font-mono">$ ./program</text>
        <text x="30" y="56" fontSize="10" fontWeight="700" style={{ fill: lit ? HW.lit : "var(--term-out)" }} className={`font-mono ${lit ? "scene-lit" : ""}`}>
          {fit(line ?? "", 26)}
        </text>
        <rect x="30" y="64" width="7" height="11" style={{ fill: "var(--term-out)" }} opacity="0.7" />
        <path d="M96 120 L90 140 H130 L124 120 Z" fill="#2a2d32" />
        <rect x="76" y="140" width="68" height="6" rx="3" fill="#3a3f47" />
      </g>
    );
  },
};

/** A keyboard. Parts: `keys` (lit while someone types). A tag is what is typed. */
export const keyboard: Picture = {
  name: "A keyboard",
  anchor: () => [110, 90],
  draw: ({ on, tag }) => {
    const lit = on("keys");
    const typed = tag("keys");
    return (
      <g>
        <rect x="4" y="54" width="212" height="76" rx="8" fill="#2a2d32" stroke="#41464d" strokeWidth="2" />
        {[0, 1, 2, 3].map((row) =>
          Array.from({ length: row === 3 ? 1 : 12 }, (_, i) => (
            <rect
              key={`${row}-${i}`}
              x={row === 3 ? 56 : 12 + i * 16.6 + (row % 2) * 5}
              y={62 + row * 16}
              width={row === 3 ? 108 : 13}
              height="12"
              rx="2"
              {...partStyle(lit && (row === 3 || (i + row) % 5 === 1), "#3b4048", "#4b5058")}
            />
          )),
        )}
        {typed && (
          <text x="110" y="42" textAnchor="middle" fontSize="10" fontWeight="700" className="fill-ink font-mono">
            {fit(typed, 24)}
          </text>
        )}
      </g>
    );
  },
};
