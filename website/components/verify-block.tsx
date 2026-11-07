import { CopyButton } from "./copy-button";
import { findBench, verifyCommand } from "./race-chart";
import versions from "@/data/versions.json";

/** The block below every benchmark: one command to reproduce it. */
export function VerifyBlock({ suite, id }: { suite: string; id: string }) {
  const cmd = `git clone ${versions.repo} && cd kavach\n${verifyCommand(suite, id)}`;
  const folder = findBench(suite, id).readme.replace(/\/README\.md$/, "");
  return (
    <div>
      <div className="mb-2.5 flex flex-wrap items-center gap-3.5 rounded-[10px] bg-code-bg px-[18px] py-[13px]">
        <span className="text-[11.5px] font-extrabold uppercase tracking-wider" style={{ color: "var(--tok-kw)" }}>
          Verify it
        </span>
        <code className="block min-w-0 overflow-x-auto font-mono text-[12.5px] text-code-ink whitespace-pre-line">{cmd}</code>
        <span className="flex-1" />
        <CopyButton text={cmd} solid />
      </div>
      <div className="flex flex-wrap gap-4 text-[13px] font-semibold">
        <a href={`${versions.repo}/tree/main/${folder}`}>benchmark source (all languages)</a>
        <a href={`${versions.repo}/tree/main/${folder}#results`}>raw numbers &amp; machine</a>
        <a href={`${versions.repo}/tree/main/benchmarks#rules`}>methodology</a>
        <a href={`${versions.repo}/tree/main/benchmarks/everyday#results`}>where Kavach loses</a>
      </div>
    </div>
  );
}
