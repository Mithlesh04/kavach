import fs from "node:fs";
import path from "node:path";
import { getSingletonHighlighter, type DecorationItem, type ShikiTransformer, type ThemedToken } from "shiki";
import { CopyButton } from "./copy-button";
import { FileTabs } from "./file-tabs";
import { Blades } from "./logo";
import type { HoverNote } from "@/lib/chapter";
import { kavachTheme } from "@/lib/code-theme";

// Names colored by where they come from, as the editor colors them: the
// compiler's marks for each sample (scripts/build-tokens.mjs), keyed by
// the code shown. They mirror --tok-imported and --tok-builtin.
type Mark = [line: number, col: number, len: number, type: string, mods: string[]];
const markColor = ([, , , type, mods]: Mark) =>
  mods.includes("imported") ? "#c3a6ff" : type === "method" && mods.includes("defaultLibrary") ? "#d7ba7d" : undefined;

/** Splits the grammar's tokens where a mark starts or ends, and colors the marked pieces. */
function markTokens(lines: ThemedToken[][], marks: Mark[]): ThemedToken[][] {
  return lines.map((tokens, line) => {
    const here = marks.filter((m) => m[0] === line && markColor(m));
    if (!here.length) return tokens;
    const out: ThemedToken[] = [];
    let col = 0;
    for (const t of tokens) {
      const start = col;
      const end = col + t.content.length;
      col = end;
      const cuts = new Set([start, end]);
      for (const [, c, len] of here) {
        if (c > start && c < end) cuts.add(c);
        if (c + len > start && c + len < end) cuts.add(c + len);
      }
      const points = [...cuts].sort((a, b) => a - b);
      for (let i = 0; i + 1 < points.length; i++) {
        const [a, b] = [points[i], points[i + 1]];
        const mark = here.find(([, c, len]) => a >= c && b <= c + len);
        out.push({
          ...t,
          content: t.content.slice(a - start, b - start),
          offset: t.offset + (a - start),
          color: mark ? markColor(mark) : t.color,
        });
      }
    }
    return out;
  });
}

let tokenMarks: Record<string, Mark[]> | undefined;
const marksFor = (code: string): Mark[] => {
  tokenMarks ??= JSON.parse(fs.readFileSync(path.join(process.cwd(), "data", "tokens.json"), "utf8")) as Record<string, Mark[]>;
  return tokenMarks[code] ?? [];
};

const semantic = (code: string): ShikiTransformer => ({
  name: "kavach-marks",
  tokens(lines) {
    const marks = marksFor(code);
    return marks.length ? markTokens(lines, marks) : lines;
  },
});

// The guard (decision 0033): highlighting comes from the repository's own
// TextMate grammar — the same file the VS Code extension uses — never from
// hand-colored spans. The theme lives in lib/code-theme.ts, shared with
// the playground's in-browser highlighter.
const grammarPath = path.join(process.cwd(), "..", "editors", "vscode", "syntaxes", "kavach.tmLanguage.json");

const theme = kavachTheme;

async function highlighter() {
  const grammar = JSON.parse(fs.readFileSync(grammarPath, "utf8"));
  return getSingletonHighlighter({
    themes: [theme],
    langs: [{ ...grammar, name: "kavach" }, "shellscript", "toml", "c", "sql", "python", "javascript", "cpp", "rust", "go"],
  });
}

/** Hover-note ranges: the first occurrence of each target substring. */
function decorations(code: string, notes: HoverNote[]): DecorationItem[] {
  const out: DecorationItem[] = [];
  for (const note of notes) {
    const start = code.indexOf(note.target);
    if (start < 0) {
      throw new Error(`hover note "${note.chip}": target ${JSON.stringify(note.target)} is not in the code`);
    }
    out.push({
      start,
      end: start + note.target.length,
      properties: {
        class: `tip tip-${note.kind}`,
        "data-chip": note.chip,
        "data-note": `${note.what} — Why: ${note.why}`,
        tabindex: "0",
      },
    });
  }
  return out;
}

const escapeHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/**
 * Code as one highlighted HTML string per line, through the repository's own
 * grammar, for widgets that handle lines one by one (the quiz's clickable
 * and reorderable lines).
 */
export async function codeLines(code: string, lang = "kavach"): Promise<string[]> {
  const hl = await highlighter();
  // The kavach grammar is registered in highlighter(); Shiki's types only know its bundled language names.
  const grammar = hl.codeToTokensBase(code, { lang: lang as never, theme });
  const marks = lang === "kavach" ? marksFor(code) : [];
  const lines = marks.length ? markTokens(grammar, marks) : grammar;
  return lines.map((tokens) =>
    tokens.map((t) => `<span style="color:${t.color ?? "inherit"}">${escapeHtml(t.content)}</span>`).join(""),
  );
}

/** Code as highlighted HTML, through the repository's own grammar; hover notes attached. */
export async function codeHtml(code: string, lang = "kavach", notes: HoverNote[] = []): Promise<string> {
  const hl = await highlighter();
  return hl.codeToHtml(code, {
    lang: lang === "sh" ? "shellscript" : lang,
    theme: "kavach-dark",
    decorations: decorations(code, notes),
    transformers: lang === "kavach" ? [semantic(code)] : [],
  });
}

const languageName: Record<string, string> = { kavach: "Kavach", toml: "TOML", c: "C", sh: "Shell", text: "Text" };
const lineCount = (code: string) => code.split("\n").length;

/** The command that printed a sample's output, as typed: a file is run by name, a project file by `kavach run`. */
const command = (file?: string) =>
  !file || !file.endsWith(".kav") ? undefined : file.includes("/") ? "kavach run" : `kavach run ${file}`;

/** How each run ended, in words, where the window shows it by color and marker. */
const spoken: Record<string, string> = {
  output: "The program ran and printed:",
  refused: "The compiler refused the program:",
  panic: "The program ran, then stopped with a panic:",
  warned: "The compiler warned, then the program ran:",
};

/** How many errors and warnings the compiler reported, for the Problems tab's count. */
const problems = (output: string) => output.split("\n").filter((l) => l.startsWith("error") || l.startsWith("warning:")).length;

/** A printed line's color: the compiler's refusal and a panic in red, a warning in yellow. */
function lineColor(line: string, outcome: string): string {
  if (outcome === "refused" || line.startsWith("panic:") || line.startsWith("error")) return "text-[var(--term-red)]";
  if (line.startsWith("warning:")) return "text-[var(--term-yellow)]";
  return "";
}

export async function CodeBlock({
  code,
  file,
  also = [],
  output,
  notes = [],
  lang = "kavach",
  outcome = "output",
  run,
}: {
  code: string;
  file?: string;
  /** The project's other files, each a tab beside `file`. */
  also?: { path: string; code: string }[];
  output?: string;
  /** What the caption shows: a run's output, the compiler refusing the
   *  code (a .fails.kav sample), a run that ends in a panic (.panics.kav),
   *  or the compiler's warnings before a run's output (.warns.kav). */
  outcome?: "output" | "refused" | "panic" | "warned";
  notes?: HoverNote[];
  lang?: string;
  /** The command that printed the output, when it isn't `kavach run`, such as `kavach check main.kav`. */
  run?: string;
}) {
  const html = await codeHtml(code, lang, notes);
  const others = await Promise.all(also.map((f) => codeHtml(f.code, lang)));
  const typed = run ?? command(file);
  const failed = outcome === "refused" || outcome === "panic";
  const label = { output: "", refused: "Compile error", panic: "Panic", warned: "Warning" }[outcome];
  const dots = (
    <span className="flex gap-2 self-center py-2.5 max-sm:hidden" aria-hidden="true">
      <span className="h-3 w-3 rounded-full bg-[var(--term-red)]" />
      <span className="h-3 w-3 rounded-full bg-[var(--term-yellow)]" />
      <span className="h-3 w-3 rounded-full bg-[var(--term-green)]" />
    </span>
  );
  /* The line numbers are a column of their own, as in an editor: only the code beside them scrolls sideways.
     Gutter (2.25rem) and gap (1rem) make the 3.25rem where the terminal's text starts too. */
  const body = (source: string, colored: string) => (
    <div className="flex py-3.5 font-mono text-[13.5px] leading-[1.8] max-sm:text-[12.5px] min-[1920px]:text-[15px]">
      <div aria-hidden="true" className="mr-4 w-9 shrink-0 select-none border-r border-code-line pr-2 text-right text-code-soft opacity-65">
        {Array.from({ length: lineCount(source) }, (_, i) => (
          <div key={i}>{i + 1}</div>
        ))}
      </div>
      <div
        className="code-body min-w-0 flex-1 pr-4 [&>pre]:m-0 [&>pre]:bg-transparent! [&>pre]:overflow-x-auto"
        dangerouslySetInnerHTML={{ __html: colored }}
      />
    </div>
  );
  const status = (source: string) => (
    <div aria-hidden="true" className="flex justify-between border-t border-code-line bg-[var(--term-bar)] px-3.5 py-1 font-mono text-[11px] text-code-soft">
      <span>{languageName[lang] ?? lang}</span>
      <span>
        {lineCount(source)} {lineCount(source) === 1 ? "line" : "lines"} · UTF-8
      </span>
    </div>
  );
  const terminal = output !== undefined && (
    <div role="group" aria-label="Terminal" className="border-t border-code-line bg-[var(--term-bg)]">
      {/* VS Code's panel: its tabs start at the code's column, past the gutter. Decoration, so screen readers skip it. */}
      <div aria-hidden="true" className="flex items-center overflow-hidden whitespace-nowrap border-b border-code-line pr-3.5 font-mono text-[11px] uppercase tracking-wider">
        <span className="w-[3.25rem] shrink-0" />
        <span className="flex items-center gap-1.5 py-1.5 pr-4 text-code-soft">
          Problems
          {problems(output) > 0 && (
            <span className={`rounded-full px-1.5 text-[10px] font-bold text-[var(--term-bg)] ${failed ? "bg-[var(--term-red)]" : "bg-[var(--term-yellow)]"}`}>
              {problems(output)}
            </span>
          )}
        </span>
        <span className="py-1.5 pr-4 text-code-soft">Output</span>
        <span className="border-b-2 border-accent py-1.5 text-code-ink">Terminal</span>
        <span className="flex-1" />
        <span className={`whitespace-nowrap pl-3 max-sm:hidden ${failed ? "text-[var(--term-red)]" : outcome === "warned" ? "text-[var(--term-yellow)]" : "text-code-soft"}`}>
          {label}
        </span>
      </div>
      <div className="flex py-3 font-mono text-[13px] leading-[1.7] max-sm:text-[12.5px] min-[1920px]:text-[14.5px]">
        {/* The gutter goes on down, with the marker an editor's terminal puts beside a command: how it ended. */}
        <div aria-hidden="true" className="mr-4 flex w-9 shrink-0 justify-end border-r border-code-line pr-2">
          {typed && (
            <span className="flex h-[1.7em] items-center">
              {failed ? (
                <span className="flex h-3.5 w-3.5 items-center justify-center rounded-full bg-[var(--term-red)] text-[9px] font-bold leading-none text-[var(--term-bg)]">✕</span>
              ) : (
                <span className="h-2.5 w-2.5 rounded-full border-2 border-[var(--term-green)]" />
              )}
            </span>
          )}
        </div>
        <div className="min-w-0 flex-1 pr-4">
          {/* What the colors and marker say, in words, for screen readers. */}
          <p className="sr-only">{run ? `${run} printed:` : spoken[outcome]}</p>
          {/* The command stays put; only what it printed scrolls sideways under it. */}
          {typed && (
            <p className="m-0">
              <span aria-hidden="true" className="select-none font-bold text-[var(--term-green)]">$ </span>
              <span className="font-semibold text-code-ink">{typed}</span>
            </p>
          )}
          {/* Line for line as the terminal prints it: a long line scrolls, never wraps, so one printed line always looks like one. */}
          <pre tabIndex={0} className="m-0 overflow-x-auto whitespace-pre text-[var(--term-out)]">
            {output.split("\n").map((line, i) => (
              <span key={i} className={`block ${lineColor(line, outcome)}`}>
                {line || " "}
              </span>
            ))}
          </pre>
        </div>
      </div>
    </div>
  );
  return (
    <figure className="code-window m-0">
      {/* An editor window: its title bar with the file's tab, numbered lines, and a status bar. */}
      <div className="overflow-hidden rounded-[10px] border border-code-line bg-code-bg shadow-[0_10px_30px_-12px_rgba(0,0,0,0.55)]">
        {also.length > 0 && file ? (
          <FileTabs
            files={[{ path: file, code }, ...also]}
            bodies={[body(code, html), ...also.map((f, i) => body(f.code, others[i]))]}
            statuses={[status(code), ...also.map((f) => status(f.code))]}
            dots={dots}
            terminal={terminal}
          />
        ) : (
          <>
            <div className="flex items-end gap-3 border-b border-code-line bg-[var(--term-bar)] pl-3.5 pr-2 max-sm:pl-2">
              {dots}
              {file && (
                <span aria-hidden="true" className="-mb-px flex min-w-0 items-center gap-1.5 rounded-t-md border border-b-0 border-code-line border-t-accent bg-code-bg px-3 py-1.5 font-mono text-xs text-code-ink">
                  <Blades size={11} />
                  <span className="truncate">{file}</span>
                </span>
              )}
              <span className="flex-1" />
              <span className="shrink-0 self-center py-1.5">
                <CopyButton text={code} />
              </span>
            </div>
            {body(code, html)}
            {terminal}
            {status(code)}
          </>
        )}
      </div>
      <figcaption className="sr-only">
        {file ? `${file}${also.length ? ` and ${also.length} more ${also.length === 1 ? "file" : "files"}` : ""}: ` : ""}
        {languageName[lang] ?? lang} code{output !== undefined ? (run ? `, then what ${run} prints` : ", then what running it prints") : ""}
      </figcaption>
    </figure>
  );
}
