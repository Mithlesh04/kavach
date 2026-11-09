import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { limitAreas, type LimitArea } from "@/lib/limits";
import { findNode } from "@/lib/nav";
import { H2, Note, P } from "./parts";

function area(id: string): LimitArea {
  const found = limitAreas.find((a) => a.id === id);
  if (!found) throw new Error(`lib/limits.ts: no area ${id}`);
  return found;
}

function Rows({ rows }: { rows: LimitArea["rows"] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">What</th>
            <th className="py-2 pr-4 font-bold">Limit or size</th>
            <th className="py-2 font-bold">Source</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.what} className="border-b border-line align-top">
              <td className="py-2 pr-4"><C>{r.what}</C></td>
              <td className="py-2 pr-4 text-muted">{r.value}</td>
              <td className="py-2"><SrcRef file={r.source} label={r.source.split("/").pop()} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The limits and sizes of one topic, on the page that explains it (from lib/limits.ts). */
export function Limits({ id }: { id: string }) {
  return (
    <>
      <Rows rows={area(id).rows} />
      <P>
        Every limit in Kavach, on one page: <Link href="/docs/tools/limits-and-sizes/">Limits and Sizes</Link>.
      </P>
    </>
  );
}

export const limitsDoc: LessonDoc = {
  lead: "Every limit and size in Kavach in one place: number ranges, text, lists, tasks, the network, JSON, AI calls, the GPU and files, each linked to the page that explains it.",
  simple: <>Like the weight limit in a lift, each part of Kavach has a size it can handle. This page lists them all together, so you can look one up without hunting through every page.</>,
  minutes: 5,
  body: (
    <>
      <P>Each table is the same one its topic&apos;s page shows, from one list, so the two always agree.</P>
      {limitAreas.map((a) => {
        const node = findNode(a.page);
        if (!node) throw new Error(`lib/limits.ts: no course page ${a.page}`);
        return (
          <section key={a.id} aria-label={a.title} className="flex flex-col gap-3">
            <H2>{a.title}</H2>
            <Rows rows={a.rows} />
            <p className="m-0 text-[14px] text-soft">
              Explained on <Link href={node.href}>{node.sub ? `${node.page.title}: ${node.sub.title}` : node.title}</Link>
            </p>
          </section>
        );
      })}
      <H2>Recursion</H2>
      <P>
        How deep a function can call itself depends on where it runs: see the table on{" "}
        <Link href="/docs/functions/recursion/">Recursion</Link>.
      </P>
      <Note>Limits that a setting can change, such as the file length, are listed with their settings on <Link href="/docs/tools/settings/">Settings</Link>.</Note>
    </>
  ),
};
