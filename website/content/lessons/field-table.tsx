import type { ReactNode } from "react";
import { C } from "@/components/inline-code";
import { typeFields } from "@/lib/ai-model";

/** Doc text with its `code` spans shown as code. */
const code = (text: string) => text.split("`").map((part, i) => (i % 2 ? <C key={i}>{part}</C> : part));

/**
 * Every field of a std.ai record, as the library declares it: its type, its
 * default (the `/// none: …` doc line), the values it takes and what it
 * does. `means` and `range` are the page's plain words for each field; a
 * field without them fails the build, so a new one is never left
 * unexplained.
 */
export function FieldTable({ type, means, range }: { type: string; means: Record<string, ReactNode>; range: Record<string, ReactNode> }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Setting</th>
            <th className="py-2 pr-4 font-bold">Type</th>
            <th className="py-2 pr-4 font-bold">Default</th>
            <th className="py-2 pr-4 font-bold">Range</th>
            <th className="py-2 font-bold">What it does</th>
          </tr>
        </thead>
        <tbody>
          {typeFields(type).map((f) => {
            if (!means[f.name] || !range[f.name]) throw new Error(`std.ai ${type} has a field "${f.name}" this page doesn't explain`);
            const given = f.doc.match(/^none: ([^;]+)/)?.[1];
            const fallback = !f.optional ? "required" : given === undefined || given === "none" ? "not sent" : code(given);
            return (
              <tr key={f.name} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{f.name}</C></td>
                <td className="py-2 pr-4 whitespace-nowrap"><C>{f.type}</C></td>
                <td className="py-2 pr-4 text-muted">{fallback}</td>
                <td className="py-2 pr-4 text-muted">{range[f.name]}</td>
                <td className="py-2 text-muted">{means[f.name]}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
