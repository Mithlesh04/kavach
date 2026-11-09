"use client";

import { useMemo, useState, type ReactNode } from "react";
import feedback from "@/content/feedback.json";
import { QuizCelebration } from "./quiz-celebration";

// Question styles rotate — pick-one, true/false, type-the-output,
// spot-the-bug, put-lines-in-order — and the cheers and nudges never
// repeat twice in a row.
/** A question's code, colored by the grammar at build time (components/quiz-block.tsx): one HTML string per line. */
export type QuizCode = { codeHtml?: string[]; linesHtml?: string[] };

type Question = (
  | { type: "spot"; prompt: ReactNode; code: string; answer: number; explain: string }
  | { type: "order"; prompt: ReactNode; lines: string[]; explain: string }
  | { type: "bool"; prompt: ReactNode; code?: string; answer: boolean; explain: string }
  | { type: "choice"; prompt: ReactNode; code?: string; options: string[]; answer: number; explain: string }
  | { type: "type-output"; prompt: ReactNode; code: string; answer: string; explain: string }
) &
  QuizCode;

function roll(pool: string[], last: number): number {
  let i = Math.floor(Math.random() * pool.length);
  if (i === last) i = (i + 1) % pool.length;
  return i;
}

function shuffled<T>(xs: T[]): T[] {
  const out = [...xs];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

const hint: Record<string, string> = {
  choice: "pick one",
  bool: "true or false?",
  "type-output": "type what it prints",
  spot: "click the buggy line",
  order: "click the lines into order",
};

export function Quiz({ questions, nextTitle }: { questions: Question[]; nextTitle?: string }) {
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState(-1);
  const [typed, setTyped] = useState("");
  const [built, setBuilt] = useState<number[]>([]);
  const [checked, setChecked] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string; explain: string } | null>(null);
  const [last, setLast] = useState({ cheer: -1, nudge: -1 });
  // First-try score: a question counts when its first Check is right.
  const [tried, setTried] = useState(false);
  const [score, setScore] = useState(0);

  const raw = questions[at];
  const pool = useMemo(
    () => (raw?.type === "order" ? shuffled(raw.lines.map((_, i) => i)) : []),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [at],
  );
  if (!raw) return null;

  // True/false renders through the choice machinery.
  const q: Exclude<Question, { type: "bool" }> =
    raw.type === "bool"
      ? {
          type: "choice",
          prompt: raw.prompt,
          code: raw.code,
          options: ["True", "False"],
          answer: raw.answer ? 0 : 1,
          explain: raw.explain,
          codeHtml: raw.codeHtml,
        }
      : raw;

  const correct =
    q.type === "choice"
      ? picked === q.answer
      : q.type === "type-output"
        ? typed.trim() === q.answer
        : q.type === "spot"
          ? picked === q.answer - 1
          : built.length === q.lines.length && built.every((v, i) => v === i);

  const check = () => {
    if ((q.type === "choice" || q.type === "spot") && picked < 0) return;
    if (q.type === "order" && built.length !== q.lines.length) return;
    const key = correct ? "cheer" : "nudge";
    const fromPool = correct ? feedback.cheers : feedback.nudges;
    const i = roll(fromPool, last[key]);
    setLast({ ...last, [key]: i });
    setMsg({ ok: correct, text: fromPool[i], explain: q.explain });
    setChecked(true);
    if (!tried) {
      setTried(true);
      if (correct) setScore(score + 1);
    }
  };

  const advance = () => {
    setAt(at + 1);
    setPicked(-1);
    setTyped("");
    setBuilt([]);
    setChecked(false);
    setMsg(null);
    setTried(false);
  };

  const clear = () => {
    setChecked(false);
    setMsg(null);
  };

  return (
    <section aria-label="Quiz" className="rounded-xl border border-line bg-panel px-6 py-5">
      <div className="mb-3 flex items-center gap-2.5">
        <span className="rounded-md bg-inv-bg px-2.5 py-1 text-[11.5px] font-extrabold tracking-wider text-inv-ink">
          QUIZ {at + 1} / {questions.length}
        </span>
        <span className="text-[13.5px] text-soft">{hint[raw.type]}</span>
      </div>

      <div className="mb-3 text-base font-bold">{q.prompt}</div>

      {(q.type === "choice" || q.type === "type-output") && q.code && (
        <pre className="mb-3.5 overflow-x-auto rounded-[10px] bg-code-bg px-4 py-3.5 font-mono text-[13px] leading-[1.75] text-code-ink">
          {q.codeHtml ? <code dangerouslySetInnerHTML={{ __html: q.codeHtml.join("\n") }} /> : <code>{q.code}</code>}
        </pre>
      )}

      {q.type === "choice" && (
        <div className="mb-3.5 flex flex-col gap-2">
          {q.options.map((label, i) => {
            let cls = "border-line bg-face";
            if (picked === i && !checked) cls = "border-[var(--ink)] bg-active";
            if (checked && i === q.answer && picked === q.answer) cls = "border-ok bg-ok-bg";
            if (checked && picked === i && i !== q.answer) cls = "border-bad bg-bad-bg";
            return (
              <button
                key={i}
                onClick={() => {
                  setPicked(i);
                  clear();
                }}
                className={`cursor-pointer rounded-lg border-[1.5px] px-4 py-[11px] text-left font-mono text-sm text-ink ${cls}`}
              >
                {label}
              </button>
            );
          })}
        </div>
      )}

      {q.type === "type-output" && (
        <div className="mb-3.5 flex flex-wrap items-center gap-3">
          <label htmlFor={`quiz-${at}`} className="text-sm font-bold">
            It prints:
          </label>
          <input
            id={`quiz-${at}`}
            value={typed}
            onChange={(e) => {
              setTyped(e.target.value);
              clear();
            }}
            placeholder="?"
            className="w-36 rounded-lg border-[1.5px] border-line bg-face px-3 py-2 font-mono text-[15px] text-ink"
          />
        </div>
      )}

      {q.type === "spot" && (
        <div className="mb-3.5 flex flex-col overflow-hidden rounded-[10px] bg-code-bg py-2">
          {q.code.split("\n").map((line, i) => {
            let cls = "";
            if (picked === i && !checked) cls = "bg-[rgba(230,162,60,0.16)]";
            if (checked && i === q.answer - 1 && picked === q.answer - 1) cls = "bg-[rgba(127,201,143,0.18)]";
            if (checked && picked === i && i !== q.answer - 1) cls = "bg-[rgba(240,138,138,0.18)]";
            return (
              <button
                key={i}
                onClick={() => {
                  setPicked(i);
                  clear();
                }}
                className={`cursor-pointer whitespace-pre border-0 bg-transparent px-4 py-1 text-left font-mono text-[13px] leading-[1.7] text-code-ink ${cls}`}
              >
                <span className="mr-3 inline-block w-5 text-right text-code-soft">{i + 1}</span>
                {q.codeHtml?.[i] !== undefined ? <span dangerouslySetInnerHTML={{ __html: q.codeHtml[i] }} /> : line}
              </button>
            );
          })}
        </div>
      )}

      {q.type === "order" && (
        <div className="mb-3.5 flex flex-col gap-3">
          <div className="min-h-[64px] rounded-[10px] bg-code-bg px-4 py-3 font-mono text-[13px] leading-[1.9] text-code-ink">
            {built.length === 0 && (
              <span className="text-code-soft">Click lines below to build the program</span>
            )}
            {built.map((lineIx, pos) => (
              <button
                key={pos}
                onClick={() => {
                  setBuilt(built.filter((_, i) => i !== pos));
                  clear();
                }}
                title="Click to remove"
                className="block cursor-pointer whitespace-pre border-0 bg-transparent p-0 text-left font-mono text-[13px] text-code-ink"
              >
                {q.linesHtml?.[lineIx] ? <span dangerouslySetInnerHTML={{ __html: q.linesHtml[lineIx] }} /> : q.lines[lineIx]}
              </button>
            ))}
          </div>
          <div className="flex flex-wrap gap-2">
            {pool
              .filter((i) => !built.includes(i))
              .map((i) => (
                <button
                  key={i}
                  onClick={() => {
                    setBuilt([...built, i]);
                    clear();
                  }}
                  className="cursor-pointer whitespace-pre rounded-lg border-[1.5px] border-code-line bg-code-bg px-3 py-1.5 text-left font-mono text-[13px] text-code-ink"
                >
                  {q.linesHtml?.[i] ? <span dangerouslySetInnerHTML={{ __html: q.linesHtml[i] }} /> : q.lines[i]}
                </button>
              ))}
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-3.5">
        <button
          onClick={check}
          className="cursor-pointer rounded-lg border-0 bg-inv-bg px-[18px] py-2.5 text-sm font-bold text-inv-ink"
        >
          Check
        </button>
        {msg && (
          <p className="m-0 text-sm font-bold" style={{ color: msg.ok ? "var(--ok)" : "var(--bad)" }}>
            {msg.text} <span className="font-medium text-muted">{msg.ok ? msg.explain : ""}</span>
          </p>
        )}
        {msg?.ok && at + 1 < questions.length && (
          <button
            onClick={advance}
            className="cursor-pointer rounded-lg border-[1.5px] border-[var(--ink)] bg-transparent px-4 py-2 text-[13.5px] font-bold text-ink"
          >
            Question {at + 2} →
          </button>
        )}
      </div>
      {msg?.ok && at + 1 === questions.length && (
        <QuizCelebration score={score} total={questions.length} nextTitle={nextTitle} />
      )}
    </section>
  );
}
