import benchmarks from "@/data/benchmarks.json";

const seriesColor: Record<string, string> = {
  kavach: "var(--ch-k)",
  cpp: "var(--ch-c)",
  python: "var(--ch-p)",
};

/**
 * The race: every bar fills at the same speed — milliseconds to scale —
 * so first to finish is fastest. Numbers come only from benchmarks.json,
 * which scripts/build-benchmarks.mjs reads from the benchmark READMEs.
 */
export function RaceChart({ suite, id }: { suite: string; id: string }) {
  const bench = findBench(suite, id);
  const max = Math.max(...bench.series.map((s) => s.value));
  const fastest = Math.min(...bench.series.map((s) => s.value));
  const cycle = 8; // seconds for the slowest bar

  return (
    <div className="rounded-xl border border-line bg-panel px-6 py-5">
      <p className="mb-3 text-[13px] text-soft">
        {bench.title} · <span className="font-mono text-[12px]">{bench.environment}</span>
      </p>
      <div className="flex flex-col gap-3.5">
        {bench.series.map((s) => {
          const widthPct = (s.value / max) * 99;
          const dur = (s.value / max) * cycle;
          const ratio = s.value / fastest;
          const verdict =
            s.value === fastest ? "· finished 1st" : `· ${trim(ratio)}× behind`;
          const verdictColor =
            s.value === fastest ? "var(--ok)" : ratio >= 5 ? "var(--bad)" : "var(--muted)";
          return (
            <div key={s.id}>
              <div className="mb-1 flex justify-between">
                <span className="text-[13px] font-bold">{s.label}</span>
                <span className="font-mono text-[12.5px]">
                  {s.value.toLocaleString("en-US")} {bench.unit}{" "}
                  <strong style={{ color: verdictColor, animation: `reveal 0.3s ease ${dur}s both` }}>
                    {verdict}
                  </strong>
                </span>
              </div>
              <div className="h-3.5 overflow-hidden rounded-[7px] bg-track">
                <div
                  role="img"
                  aria-label={`${s.label}: ${s.value.toLocaleString("en-US")} ${bench.unit}`}
                  className="h-3.5 rounded-l-[7px] rounded-r-[4px]"
                  style={{
                    width: `${widthPct}%`,
                    background: seriesColor[s.color] ?? "var(--muted)",
                    animation: `race ${dur}s linear both`,
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function trim(x: number): string {
  return x >= 10 ? String(Math.round(x)) : x.toFixed(1).replace(/\.0$/, "");
}

export type Bench = {
  title: string;
  unit: string;
  series: { id: string; label: string; color: string; value: number }[];
  verify: string;
  readme: string;
  environment: string;
};

type Suite = { environment: string; benchmarks: Record<string, Omit<Bench, "environment">> };

/** The one lookup into benchmarks.json; an unknown id fails the build. */
export function findBench(suite: string, id: string): Bench {
  const s = (benchmarks.suites as Record<string, Suite>)[suite];
  const bench = s?.benchmarks[id];
  if (!bench) throw new Error(`unknown benchmark ${suite}/${id} — add it to data/benchmark-sources.json`);
  return { ...bench, environment: s.environment };
}

export function verifyCommand(suite: string, id: string): string {
  return findBench(suite, id).verify;
}
