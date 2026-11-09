import type { ReactNode } from "react";
import { SrcRef } from "./src-ref";
import type { SmartThing } from "@/content/smart";

/** "What Kavach does for you": the smart things a page teaches, each with its proof. */
export function SmartBox({
  things,
  title = "What Kavach does for you",
  extra,
}: {
  things: SmartThing[];
  title?: string;
  /** Something to show beside a thing's proof, such as a link to the page that teaches it. */
  extra?: (t: SmartThing) => ReactNode;
}) {
  return (
    <section aria-label={title} className="mt-8 rounded-xl border border-line bg-accent-soft px-6 py-5">
      <h2 className="m-0 mb-3 text-lg font-extrabold">{title}</h2>
      <ul className="m-0 flex list-none flex-col gap-4 p-0">
        {things.map((t) => (
          <li key={t.title}>
            <p className="m-0 text-[15.5px] font-bold text-ink">{t.title}</p>
            <p className="m-0 mt-0.5 text-[15px] leading-relaxed text-muted">{t.what}</p>
            <p className="m-0 mt-2 flex flex-wrap items-center gap-3">
              <SrcRef file={t.proof.file} label={t.proof.label} />
              {extra?.(t)}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}
