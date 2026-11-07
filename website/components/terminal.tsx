import { codeHtml } from "./code-block";
import { CopyButton } from "./copy-button";
import type { HoverNote } from "@/lib/chapter";

/**
 * A terminal window: a title bar, then `$ command` lines, each followed by
 * what it printed, and a cursor waiting at the end. `session` reads as the
 * terminal does: lines starting with `$ ` are commands, the rest output (a
 * sample's .expect for a project with a `runs` file has exactly this shape).
 * With `commands`, every line is a command, and `# …` after one is a note.
 * Output that is Kavach code (`kavach outline`, `kavach context`) is
 * highlighted with the repository's grammar; other output stays plain.
 */
export async function Terminal({
  session,
  commands = false,
  outputLang = "text",
  title = "terminal",
  notes = [],
}: {
  session: string;
  commands?: boolean;
  outputLang?: "kavach" | "text";
  title?: string;
  notes?: HoverNote[];
}) {
  const steps: { cmd: string; note?: string; out: string[] }[] = [];
  for (const line of session.replace(/\r\n/g, "\n").trimEnd().split("\n")) {
    if (commands || line.startsWith("$ ")) {
      const text = commands ? line : line.slice(2);
      const hash = commands ? text.indexOf(" #") : -1;
      steps.push(hash >= 0 ? { cmd: text.slice(0, hash).trimEnd(), note: text.slice(hash + 1), out: [] } : { cmd: text, out: [] });
    } else if (steps.length) {
      steps[steps.length - 1].out.push(line);
    } else {
      throw new Error(`Terminal: output before the first command: ${JSON.stringify(line)}`);
    }
  }

  // Each hover note goes on the first output that holds its target.
  const outputs = steps.map((s) => s.out.join("\n"));
  const placed = outputs.map(() => [] as HoverNote[]);
  for (const note of notes) {
    const i = outputs.findIndex((o) => o.includes(note.target));
    if (i < 0) throw new Error(`Terminal: hover note "${note.chip}" target ${JSON.stringify(note.target)} is in no output`);
    placed[i].push(note);
  }
  const rendered = await Promise.all(
    outputs.map((o, i) => (o && outputLang === "kavach" ? codeHtml(o, "kavach", placed[i]) : Promise.resolve(null))),
  );
  // Notes after commands start in one column, two spaces past the longest command.
  const width = Math.max(...steps.map((s) => s.cmd.length)) + 2;

  return (
    <figure className="code-window m-0 overflow-hidden rounded-[10px] border border-code-line bg-[var(--term-bg)] shadow-[0_10px_30px_-12px_rgba(0,0,0,0.55)]">
      <div className="flex items-center gap-2 rounded-t-[10px] border-b border-code-line bg-[var(--term-bar)] px-3.5 py-2">
        <span aria-hidden="true" className="h-3 w-3 rounded-full bg-[var(--term-red)]" />
        <span aria-hidden="true" className="h-3 w-3 rounded-full bg-[var(--term-yellow)]" />
        <span aria-hidden="true" className="h-3 w-3 rounded-full bg-[var(--term-green)]" />
        <span className="flex-1 text-center font-mono text-xs text-code-soft">{title}</span>
        <CopyButton text={steps.map((s) => s.cmd).join("\n")} />
      </div>
      <div className="w-0 min-w-full overflow-x-auto px-4 py-3.5 font-mono text-[13.5px] leading-[1.75]">
        {steps.map((s, i) => (
          <div key={i} className={i > 0 && steps[i - 1].out.length ? "mt-2.5" : ""}>
            <div className="flex gap-2.5 whitespace-pre">
              <span aria-hidden="true" className="select-none font-bold text-[var(--term-green)]">$</span>
              <span className="font-semibold text-code-ink">
                {s.cmd}
                {s.note ? (
                  <span className="font-normal text-code-soft">
                    {" ".repeat(width - s.cmd.length)}
                    {s.note}
                  </span>
                ) : null}
              </span>
            </div>
            {rendered[i] ? (
              <div
                className="code-body [&>pre]:m-0 [&>pre]:bg-transparent! [&>pre]:whitespace-pre"
                dangerouslySetInnerHTML={{ __html: rendered[i]! }}
              />
            ) : s.out.length ? (
              <pre className="m-0 whitespace-pre text-[var(--term-out)]">{outputs[i]}</pre>
            ) : null}
          </div>
        ))}
        <div className={`flex items-center gap-2.5 ${steps.at(-1)?.out.length ? "mt-2.5" : ""}`} aria-hidden="true">
          <span className="select-none font-bold text-[var(--term-green)]">$</span>
          <span className="inline-block h-[1.05em] w-[0.6em] bg-code-ink" style={{ animation: "termblink 1.1s steps(1) infinite" }} />
        </div>
      </div>
    </figure>
  );
}
