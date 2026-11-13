import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import type { LessonDoc } from "@/lib/chapter";
import { allNodes } from "@/lib/nav";
import { samplePages, sampleNames } from "@/lib/sample-pages";
import { hasContent } from "@/lib/site";
import { kindOf } from "@/lib/sample-kinds.mjs";
import { H2, P } from "./parts";
import { Cell } from "./spec-table";

// ---------- Reference › Compiler Messages ----------
//
// Every error, warning and panic message the site's samples produce: read
// from their .expect files, which scripts/check-samples.mjs holds to what
// the real kavach prints on every build. Each links to the page that shows
// it, the first in course order.

type Message = { kind: "error" | "warning" | "panic"; text: string; key?: string };

const samplesDir = path.join(process.cwd(), "content", "samples");

/** The message lines of one sample's checked output. */
function linesOf(name: string): string[] {
  const kind = kindOf(name.endsWith(".project") ? name : `${name}.kav`);
  const expect = path.join(samplesDir, `${name}.expect`);
  if (!fs.existsSync(expect)) return [];
  const lines = fs.readFileSync(expect, "utf8").replace(/\r\n/g, "\n").split("\n");
  const story = fs.existsSync(path.join(samplesDir, `${name}.project`, "runs"));
  if (kind === "fails") return lines.slice(0, 1);
  if (kind === "warns" || kind === "panics" || story) return lines.filter((l) => /^(error|warning|panic): /.test(l));
  return [];
}

function messages(): Message[] {
  const order = new Map(allNodes().map((n, i) => [n.key, i]));
  const pages = samplePages();
  const firstPage = (name: string) =>
    (pages.get(name) ?? [])
      .filter((k) => hasContent(k) && order.has(k))
      .sort((a, b) => order.get(a)! - order.get(b)!)[0];
  const seen = new Set<string>();
  const out: Message[] = [];
  for (const name of sampleNames()) {
    const key = firstPage(name);
    if (!key) continue; // a sample no page shows has no place to explain it
    for (const line of linesOf(name)) {
      const [, kind, text] = line.match(/^(error|warning|panic): (.*)$/) ?? [];
      if (!kind || seen.has(line)) continue;
      seen.add(line);
      out.push({ kind: kind as Message["kind"], text, key });
    }
  }
  return out.sort((a, b) => order.get(a.key!)! - order.get(b.key!)!);
}

function MessageTable({ rows }: { rows: Message[] }) {
  const nodes = new Map(allNodes().map((n) => [n.key, n]));
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Kind</th>
            <th className="py-2 pr-4 font-bold">Message</th>
            <th className="py-2 font-bold">Shown in</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((m) => {
            const node = nodes.get(m.key!)!;
            return (
              <tr key={m.kind + m.text} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-soft">{m.kind}</td>
                <td className="py-2 pr-4 text-muted"><Cell text={m.text} /></td>
                <td className="py-2"><Link href={node.href}>{node.title}</Link></td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export const messagesRefDoc: LessonDoc = {
  lead: "The compiler's errors and warnings, and the panics of a running program, as the samples on this site show them: each with the page that explains it.",
  minutes: 10,
  body: <Messages />,
};

/** Rendered, not built when the module loads: the list needs every lesson, and this one is among them. */
function Messages() {
  const all = messages();
  const parts = [...new Set(all.map((m) => allNodes().find((n) => n.key === m.key)!.part.title))];
  return (
    <>
      <P>
        Each line below was printed by the real compiler, or by a program it built, when this site was built: the
        samples&apos; output is checked on every build. A message says what is wrong and, most of the time, what to write
        instead. The compiler has more messages than the samples show.
      </P>
      {parts.map((title) => (
        <div key={title} className="flex flex-col gap-3">
          <H2>{title}</H2>
          <MessageTable rows={all.filter((m) => allNodes().find((n) => n.key === m.key)!.part.title === title)} />
        </div>
      ))}
    </>
  );
}
