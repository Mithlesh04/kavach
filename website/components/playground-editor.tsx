"use client";

// The playground (decision 0078): a program typed here is checked by the
// real compiler, built for WebAssembly and running in this browser —
// nothing is sent anywhere. It looks like the site's other code windows:
// the same grammar colors the code (through Shiki's JavaScript engine,
// since there is no build step for what a visitor types), and the same
// terminal shows what `kavach check` says.

import { useCallback, useEffect, useRef, useState } from "react";
import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import { kavachTheme } from "@/lib/code-theme";
import { runWasi } from "@/lib/wasi-run";

export type Starter = { name: string; code: string };

type Message = {
  severity: "error" | "warning";
  start: number;
  end: number;
  text: string;
  rendered: string;
};

type State =
  | { kind: "loading" }
  | { kind: "missing" }
  | { kind: "checked"; messages: Message[] }
  | { kind: "broken"; why: string };

const base = () => process.env.NEXT_PUBLIC_BASE_PATH ?? "";

let enginePromise: Promise<WebAssembly.Module> | null = null;
function engine(): Promise<WebAssembly.Module> {
  enginePromise ??= (async () => {
    const res = await fetch(`${base()}/playground/engine.wasm`);
    if (!res.ok) throw new Error(`status ${res.status}`);
    return WebAssembly.compile(await res.arrayBuffer());
  })();
  return enginePromise;
}

let coloristPromise: Promise<HighlighterCore> | null = null;
function colorist(): Promise<HighlighterCore> {
  coloristPromise ??= (async () => {
    const res = await fetch(`${base()}/playground/kavach.tmLanguage.json`);
    if (!res.ok) throw new Error(`status ${res.status}`);
    const grammar = await res.json();
    return createHighlighterCore({
      themes: [kavachTheme],
      langs: [{ ...grammar, name: "kavach" }],
      engine: createJavaScriptRegexEngine({ forgiving: true }),
    });
  })();
  return coloristPromise;
}

/// 1-based line of a byte offset in `src`.
function lineOf(src: string, offset: number): number {
  const before = new TextDecoder().decode(new TextEncoder().encode(src).subarray(0, offset));
  return (before.match(/\n/g)?.length ?? 0) + 1;
}

/** A terminal line's color, as the site's code windows color it. */
function lineColor(line: string): string {
  if (line.startsWith("error")) return "text-[var(--term-red)]";
  if (line.startsWith("warning")) return "text-[var(--term-yellow)]";
  return "";
}

const mono = "font-mono text-[13.5px] leading-[1.8] max-sm:text-[12.5px]";

export function PlaygroundEditor({ starters }: { starters: Starter[] }) {
  const [source, setSource] = useState(starters[0]?.code ?? "");
  const [colored, setColored] = useState<string | null>(null);
  const [state, setState] = useState<State>({ kind: "loading" });
  const run = useRef(0);
  const shineRun = useRef(0);
  const preRef = useRef<HTMLDivElement>(null);

  const check = useCallback(async (src: string) => {
    const mine = ++run.current;
    try {
      const mod = await engine();
      const result = await runWasi(mod, new TextEncoder().encode(src));
      if (run.current !== mine) return; // A newer check took over.
      const answer = JSON.parse(result.stdout) as { messages?: Message[]; failed?: string };
      if (answer.messages) setState({ kind: "checked", messages: answer.messages });
      else setState({ kind: "broken", why: answer.failed ?? "no answer" });
    } catch (e) {
      if (run.current !== mine) return;
      const why = e instanceof Error ? e.message : String(e);
      setState(why.startsWith("status 4") ? { kind: "missing" } : { kind: "broken", why });
    }
  }, []);

  useEffect(() => {
    const soon = setTimeout(() => void check(source), 350);
    return () => clearTimeout(soon);
  }, [source, check]);

  // Colors follow every keystroke; until the grammar is here, the code
  // shows plain in the same metrics, so nothing jumps.
  useEffect(() => {
    const mine = ++shineRun.current;
    void colorist()
      .then((hl) => {
        if (shineRun.current !== mine) return;
        setColored(hl.codeToHtml(source, { lang: "kavach", theme: "kavach-dark" }));
      })
      .catch(() => {});
  }, [source]);

  // Tab indents instead of leaving the editor.
  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key !== "Tab") return;
    e.preventDefault();
    const area = e.currentTarget;
    const { selectionStart: at, selectionEnd: to, value } = area;
    setSource(value.slice(0, at) + "    " + value.slice(to));
    requestAnimationFrame(() => area.setSelectionRange(at + 4, at + 4));
  };

  const lines = source.split("\n").length;
  const messages = state.kind === "checked" ? state.messages : [];
  const errors = messages.filter((m) => m.severity === "error").length;
  const clean = state.kind === "checked" && messages.length === 0;
  const label =
    state.kind === "checked" ? (errors > 0 ? "Compile error" : messages.length > 0 ? "Warning" : "No problems") : "";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        {starters.map((s) => (
          <button
            key={s.name}
            type="button"
            onClick={() => setSource(s.code)}
            className="rounded-full border border-line bg-panel px-3.5 py-1.5 text-[13.5px] font-semibold text-muted hover:text-ink"
          >
            {s.name}
          </button>
        ))}
      </div>

      <div className="code-window overflow-hidden rounded-[10px] border border-code-line bg-code-bg shadow-[0_10px_30px_-12px_rgba(0,0,0,0.55)]">
        {/* The editor window's title bar, as every code window on the site. */}
        <div className="flex items-end gap-3 border-b border-code-line bg-[var(--term-bar)] pl-3.5 pr-2 max-sm:pl-2">
          <span className="flex gap-2 self-center py-2.5 max-sm:hidden" aria-hidden="true">
            <span className="h-3 w-3 rounded-full bg-[var(--term-red)]" />
            <span className="h-3 w-3 rounded-full bg-[var(--term-yellow)]" />
            <span className="h-3 w-3 rounded-full bg-[var(--term-green)]" />
          </span>
          <span
            aria-hidden="true"
            className="-mb-px flex min-w-0 items-center rounded-t-md border border-b-0 border-code-line border-t-accent bg-code-bg px-3 py-1.5 font-mono text-xs text-code-ink"
          >
            playground.kav
          </span>
          <span className="flex-1" />
          <span className="shrink-0 self-center py-1.5 pr-1.5 font-mono text-[11px] text-code-soft max-sm:hidden">
            edit me
          </span>
        </div>

        {/* The code: numbered lines, grammar colors behind, the caret on a see-through textarea in front. */}
        <div className={`flex py-3.5 ${mono}`}>
          <div
            aria-hidden="true"
            className="mr-4 w-9 shrink-0 select-none border-r border-code-line pr-2 text-right text-code-soft opacity-65"
          >
            {Array.from({ length: lines }, (_, i) => (
              <div key={i}>{i + 1}</div>
            ))}
          </div>
          {/* The textarea owns the scrolling; the colored layer under it follows. */}
          <div className="relative min-w-0 flex-1 overflow-hidden">
            <div ref={preRef} className="overflow-hidden">
              <pre aria-hidden="true" className="code-body pointer-events-none m-0 min-h-[1.8em] w-max min-w-full bg-transparent pr-4">
                {colored ? (
                  <code dangerouslySetInnerHTML={{ __html: colored.replace(/^<pre[^>]*><code[^>]*>/, "").replace(/<\/code><\/pre>\s*$/, "") }} />
                ) : (
                  <code style={{ color: "#e7e0d4" }}>{source}</code>
                )}
                {"\n"}
              </pre>
            </div>
            <textarea
              value={source}
              onChange={(e) => setSource(e.target.value)}
              onKeyDown={onKeyDown}
              onScroll={(e) => {
                if (preRef.current) preRef.current.scrollLeft = e.currentTarget.scrollLeft;
              }}
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              wrap="off"
              aria-label="Kavach program (checked as you type)"
              className={`absolute inset-0 h-full w-full resize-none overflow-auto whitespace-pre border-0 bg-transparent pr-4 text-transparent caret-[#e7e0d4] outline-none ${mono}`}
            />
          </div>
        </div>

        {/* The terminal, exactly as the site's code windows show one. */}
        <div role="group" aria-label="Terminal" className="border-t border-code-line bg-[var(--term-bg)]">
          <div
            aria-hidden="true"
            className="flex items-center overflow-hidden whitespace-nowrap border-b border-code-line pr-3.5 font-mono text-[11px] uppercase tracking-wider"
          >
            <span className="w-[3.25rem] shrink-0" />
            <span className="flex items-center gap-1.5 py-1.5 pr-4 text-code-soft">
              Problems
              {messages.length > 0 && (
                <span
                  className={`rounded-full px-1.5 text-[10px] font-bold text-[var(--term-bg)] ${errors > 0 ? "bg-[var(--term-red)]" : "bg-[var(--term-yellow)]"}`}
                >
                  {messages.length}
                </span>
              )}
            </span>
            <span className="border-b-2 border-accent py-1.5 text-code-ink">Terminal</span>
            <span className="flex-1" />
            <span
              className={`whitespace-nowrap pl-3 max-sm:hidden ${errors > 0 ? "text-[var(--term-red)]" : messages.length > 0 ? "text-[var(--term-yellow)]" : "text-code-soft"}`}
            >
              {label}
            </span>
          </div>
          <div aria-live="polite" className="flex py-3 font-mono text-[13px] leading-[1.7] max-sm:text-[12.5px]">
            <div aria-hidden="true" className="mr-4 flex w-9 shrink-0 justify-end border-r border-code-line pr-2">
              <span className="flex h-[1.7em] items-center">
                {errors > 0 ? (
                  <span className="flex h-3.5 w-3.5 items-center justify-center rounded-full bg-[var(--term-red)] text-[9px] font-bold leading-none text-[var(--term-bg)]">
                    ✕
                  </span>
                ) : (
                  <span className="h-2.5 w-2.5 rounded-full border-2 border-[var(--term-green)]" />
                )}
              </span>
            </div>
            <div className="min-w-0 flex-1 pr-4">
              <p className="m-0">
                <span aria-hidden="true" className="select-none font-bold text-[var(--term-green)]">
                  ${" "}
                </span>
                <span className="font-semibold text-code-ink">kavach check playground.kav</span>
              </p>
              {state.kind === "loading" && <p className="m-0 text-code-soft">loading the compiler…</p>}
              {state.kind === "missing" && (
                <p className="m-0 text-[var(--term-yellow)]">
                  the engine isn&apos;t built here yet: run `npm run playground` in website/
                </p>
              )}
              {state.kind === "broken" && <p className="m-0 text-[var(--term-red)]">the check failed: {state.why}</p>}
              {clean && (
                <p className="m-0 text-[var(--term-green)]">✓ no errors, no warnings — the program checks clean</p>
              )}
              {messages.length > 0 && (
                <pre tabIndex={0} className="m-0 overflow-x-auto whitespace-pre text-[var(--term-out)]">
                  {messages
                    .flatMap((m) => m.rendered.trimEnd().split("\n"))
                    .map((line, i) => (
                      <span key={i} className={`block ${lineColor(line)}`}>
                        {line || " "}
                      </span>
                    ))}
                </pre>
              )}
            </div>
          </div>
        </div>

        {/* The status bar, with the first problem's place like an editor shows it. */}
        <div
          aria-hidden="true"
          className="flex justify-between border-t border-code-line bg-[var(--term-bar)] px-3.5 py-1 font-mono text-[11px] text-code-soft"
        >
          <span>Kavach</span>
          <span>
            {messages.length > 0 && `${errors} ✕ · line ${lineOf(source, messages[0].start)} · `}
            {lines} {lines === 1 ? "line" : "lines"} · UTF-8
          </span>
        </div>
      </div>
    </div>
  );
}
