import Link from "next/link";
import { RaceChart } from "@/components/race-chart";
import { winsAgainstCpp } from "@/components/ratio";
import { SrcRef } from "@/components/src-ref";
import { VerifyBlock } from "@/components/verify-block";
import benchmarks from "@/data/benchmarks.json";

export const metadata = {
  title: "Benchmarks",
  description: "Every Kavach benchmark in one place: whole programs on one machine against C++ and Python, wins and losses, each with one command to reproduce it.",
};

// Suites in reading order, each with the course page that explains how Kavach gets there.
const order: [suite: string, explained: string, href: string][] = [
  ["loops", "Loops on all cores", "/docs/speed/loops-on-all-cores/"],
  ["math", "Loops on all cores", "/docs/speed/loops-on-all-cores/"],
  ["chains", "Method chains", "/docs/types/method-chains/"],
  ["everyday", "Functions as values", "/docs/functions/functions-as-values/"],
  ["lookups", "Maps", "/docs/collections/maps/"],
  ["sizes", "As data grows", "/docs/speed/as-data-grows/"],
  ["json", "JSON", "/docs/standard-library/json/"],
  ["bytes", "Byte-packed lists", "/docs/speed/byte-packed-lists/"],
  ["tasks", "Tasks", "/docs/concurrency/tasks/"],
  ["channels", "Channels", "/docs/concurrency/channels/"],
  ["net", "HTTP", "/docs/networks/http/"],
  ["net-clients", "Tasks", "/docs/concurrency/tasks/"],
  ["gpu", "On the GPU", "/docs/speed/gpu/"],
];

type Suite = { title: string; environment: string; measured: string | null; benchmarks: Record<string, unknown> };
const suites = benchmarks.suites as Record<string, Suite>;

export default function BenchmarksPage() {
  const { wins, total } = winsAgainstCpp();
  const missing = Object.keys(suites).filter((s) => !order.some(([o]) => o === s));
  if (missing.length) throw new Error(`benchmarks page: suites not in its order: ${missing.join(", ")}`);
  const count = Object.values(suites).reduce((n, s) => n + Object.keys(s.benchmarks).length, 0);

  return (
    <div className="px-10 pb-12 pt-[30px] max-md:px-5 max-sm:px-4 max-sm:pt-5" data-pagefind-meta="title:Benchmarks">
      <h1 className="m-0 mb-2 text-[38px] font-extrabold tracking-tight max-md:text-3xl">Benchmarks</h1>
      <p className="m-0 mb-6 max-w-[64ch] text-[17px] leading-relaxed text-muted">
        {count} whole programs, each written in Kavach, C++ and often Python, run on one machine. Kavach is faster than
        C++ in <strong className="text-ink">{wins} of {total}</strong> of them, and the others are here too.
      </p>

      <section aria-label="How these are measured" className="mb-8 rounded-xl border border-line bg-panel px-6 py-5 text-[15px] leading-relaxed">
        <h2 className="m-0 mb-2 text-lg font-extrabold">How these are measured</h2>
        <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5 text-muted">
          <li>The same algorithm in every language, written the way a fluent programmer would.</li>
          <li>Whole programs, start to finish, on the same machine; the best of several runs.</li>
          <li>Every number is read from the benchmark&apos;s README, where each run is recorded, never typed into this page.</li>
          <li>Each suite ends with one command to run it yourself.</li>
          <li>
            In a race, every bar fills at the <strong className="text-ink">same speed</strong>: milliseconds to scale, so the
            first to finish is the fastest.
          </li>
        </ul>
        <div className="mt-3"><SrcRef file="benchmarks/README.md" label="the rules, the code, the raw numbers" /></div>
      </section>

      <nav aria-label="Suites" className="mb-10 flex flex-wrap gap-2">
        {order.map(([id]) => (
          <a key={id} href={`#${id}`} className="rounded-full border border-line bg-panel px-3.5 py-1.5 text-[13px] font-semibold text-ink no-underline">
            {suites[id].title}
          </a>
        ))}
      </nav>

      {order.map(([id, explained, href]) => {
        const s = suites[id];
        const ids = Object.keys(s.benchmarks);
        return (
          <section key={id} id={id} aria-label={s.title} className="mb-12 scroll-mt-4">
            <h2 className="m-0 mb-1 text-[26px] font-extrabold tracking-tight">{s.title}</h2>
            <p className="m-0 mb-4 text-[13px] text-soft">
              {s.environment}
              {s.measured ? ` · measured ${s.measured}` : ""} ·{" "}
              <Link href={href}>how Kavach does it: {explained}</Link>
            </p>
            <div className="flex flex-col gap-6">
              {ids.map((b) => (
                <RaceChart key={b} suite={id} id={b} />
              ))}
              <VerifyBlock suite={id} id={ids[0]} />
            </div>
          </section>
        );
      })}
    </div>
  );
}
