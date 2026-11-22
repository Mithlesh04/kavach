import { C, withCode } from "@/components/inline-code";
import { typeFields } from "@/lib/ai-model";

/**
 * A std.ai record's fields, as the library declares them, with each
 * field's own doc line. `extra` gives the page's words for a field with
 * no doc line; a field with neither fails the build.
 */
export function AiFields({ type, extra = {} }: { type: string; extra?: Record<string, string> }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            <th className="py-2 pr-4 font-bold">Field</th>
            <th className="py-2 pr-4 font-bold">Type</th>
            <th className="py-2 font-bold">What it holds</th>
          </tr>
        </thead>
        <tbody>
          {typeFields(type).map((f) => {
            const says = f.doc || extra[f.name];
            if (!says) throw new Error(`std.ai ${type}.${f.name} has no doc line and the page gives none`);
            return (
              <tr key={f.name} className="border-b border-line align-top">
                <td className="py-2 pr-4"><C>{f.name}</C></td>
                <td className="py-2 pr-4 whitespace-nowrap"><C>{f.optional ? `${f.type}?` : f.type}</C></td>
                <td className="py-2 text-muted">{withCode(says)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
