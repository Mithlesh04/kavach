import benchmarks from "@/data/benchmarks.json";
import { findBench } from "./race-chart";

/**
 * How many times faster (or slower) one series is than another, from
 * data/benchmarks.json — so no page carries a typed number that the next
 * benchmark run would make stale.
 */
export function ratioOf(suite: string, id: string, oursId = "kavach", vs = "cpp") {
  const bench = findBench(suite, id);
  // No page may call a race that isn't like for like "N× faster".
  if (bench.notLikeForLike) throw new Error(`benchmark ${suite}/${id} is not like for like (${bench.notLikeForLike}): no ratio may be shown`);
  const ours = bench.series.find((s) => s.id === oursId);
  const theirs = bench.series.find((s) => s.id === vs);
  if (!ours || !theirs) throw new Error(`benchmark ${suite}/${id} has no ${oursId} or ${vs} series`);
  if (ours.value === null || theirs.value === null) throw new Error(`benchmark ${suite}/${id}: a side is still to measure, so no ratio may be shown`);
  const [o, t] = [ours.value, theirs.value];
  const faster = o <= t;
  const r = faster ? t / o : o / t;
  return { shown: r >= 10 ? Math.round(r).toString() : r.toFixed(1), faster, ours: { ...ours, value: o }, theirs: { ...theirs, value: t } };
}

/** "10× faster than C++ -O2", in a sentence. */
export function Ratio({ suite, id, ours = "kavach", vs = "cpp" }: { suite: string; id: string; ours?: string; vs?: string }) {
  const r = ratioOf(suite, id, ours, vs);
  return (
    <strong className="text-ink">
      {r.shown}× {r.faster ? "faster" : "slower"} than {r.theirs.label}
    </strong>
  );
}

/**
 * Of the published benchmarks with a C++ side that are like for like and
 * measured on both sides, how many Kavach wins. `left` counts those that
 * aren't comparable; `waiting`, those with a side still to measure.
 */
export function winsAgainstCpp(): { wins: number; total: number; left: number; waiting: number } {
  let wins = 0;
  let total = 0;
  let left = 0;
  let waiting = 0;
  for (const [suiteId, suite] of Object.entries(benchmarks.suites)) {
    for (const id of Object.keys(suite.benchmarks)) {
      const bench = findBench(suiteId, id);
      const k = bench.series.find((s) => s.id === "kavach");
      const c = bench.series.find((s) => s.id === "cpp");
      if (!k || !c) continue;
      if (bench.notLikeForLike) {
        left++;
        continue;
      }
      if (k.value === null || c.value === null) {
        waiting++;
        continue;
      }
      total++;
      if (k.value < c.value) wins++;
    }
  }
  return { wins, total, left, waiting };
}
