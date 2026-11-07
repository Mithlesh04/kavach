import { C } from "@/components/inline-code";
import { specTables } from "@/lib/spec-tables";

/** A spec table cell: `code` in backticks becomes code; outside code, ** bold marks are dropped and a [link](url) keeps its text. */
export function Cell({ text }: { text: string }) {
  const parts = text.split("`");
  const plain = (p: string) => p.replace(/\*\*/g, "").replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
  return <>{parts.map((p, i) => (i % 2 ? <C key={i}>{p}</C> : plain(p)))}</>;
}

export function Table({ header, rows, wrapFirst = false }: { header: string[]; rows: string[][]; wrapFirst?: boolean }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {header.map((h) => (
              <th key={h} className="py-2 pr-4 font-bold">{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i} className="border-b border-line align-top">
              {r.map((c, j) => (
                <td key={j} className={`py-2 pr-4 ${j > 0 ? "text-muted" : wrapFirst ? "" : "whitespace-nowrap"}`}>
                  <Cell text={c} />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/**
 * The table of a spec section whose header starts with `first`, under
 * `heading` when given: the functions of a module, as the spec lists them.
 * A bare file name is in `docs/spec/14-standard-library/`.
 */
export function ModuleTable({ file, number, first, heading }: { file: string; number: string; first: string; heading?: string }) {
  const table = specTables(file.includes("/") ? file : `14-standard-library/${file}`, number).find(
    (t) => t.header[0] === first && (heading === undefined || t.heading === heading),
  );
  if (!table) throw new Error(`spec ${number}: no table starting with ${first}${heading ? ` under ${heading}` : ""}`);
  return <Table header={table.header} rows={table.rows} wrapFirst />;
}
