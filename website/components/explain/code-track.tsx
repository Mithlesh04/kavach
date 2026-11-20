// The code beside an explainer: the checked sample, colored by the
// repository's own grammar (codeLines in components/code-block.tsx, run on
// the server), numbered from 1, with the line this step runs marked by a
// bar and an arrow, not by color alone; maybe a badge on each line where a
// name is used for the last time. And what the program printed.

/**
 * The code, one highlighted HTML string per line, with line `line` (from 0)
 * marked. `lastUse` puts a small badge after a line for each name whose
 * last use it is, as the compiler found while compiling; on a narrow
 * screen a badge wraps under its line instead of widening anything.
 */
export function CodeLines({ html, line, lastUse }: { html: string[]; line: number; lastUse?: Record<number, string[]> }) {
  return (
    <pre className="m-0 overflow-x-auto rounded-lg bg-code-bg py-2.5 font-mono text-[13px] leading-relaxed text-code-ink">
      {html.map((l, i) => {
        const now = i === line;
        const names = lastUse?.[i] ?? [];
        return (
          <div
            key={i}
            className={`flex min-w-full items-center border-l-[3px] pr-3 ${names.length ? "flex-wrap" : "w-max"} ${now ? "border-[var(--tok-kw)] bg-[color-mix(in_srgb,var(--tok-kw)_16%,transparent)] font-bold" : "border-transparent"}`}
            aria-current={now ? "step" : undefined}
          >
            <span className={`w-9 shrink-0 select-none pr-2 text-right ${now ? "text-[var(--tok-kw)]" : "text-code-soft"}`} aria-hidden="true">
              {now ? "▶" : i + 1}
            </span>
            <span className="shrink-0 whitespace-pre" dangerouslySetInnerHTML={{ __html: l || " " }} />
            {names.map((n) => (
              <span
                key={n}
                title={`${n}: last use, marked by the compiler while compiling`}
                aria-label={`${n}: last use, marked by the compiler while compiling`}
                className="ml-2 shrink-0 rounded border border-code-border px-1.5 font-sans text-[10.5px] font-normal leading-[1.6] text-code-soft"
              >
                {n}: last use
              </span>
            ))}
          </div>
        );
      })}
    </pre>
  );
}

/** What the program printed: so far, or all of it, as checked against a real run. */
export function Printed({ text, label }: { text: string; label: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[11.5px] text-soft">{label}</span>
      <pre className="m-0 min-h-[2.2em] overflow-x-auto rounded-lg border border-line bg-bg px-3 py-1.5 font-mono text-[12.5px] text-ink">{text}</pre>
    </div>
  );
}
