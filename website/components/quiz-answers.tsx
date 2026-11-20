import type { QuizQuestion } from "@/lib/chapter";
import { quizAnswer } from "@/lib/quiz-text";

/**
 * Every question of a page's quiz with its answer, in the page's HTML and
 * closed until opened, so it spoils nothing. Readers can review at once,
 * and search engines and AI tools, which don't run the quiz, read it.
 */
export function QuizAnswers({ questions }: { questions: QuizQuestion[] }) {
  return (
    <details className="mt-4 rounded-lg border border-line px-4 py-3" data-pagefind-ignore>
      <summary className="cursor-pointer text-[14.5px] font-bold">All questions, with answers</summary>
      <ol className="m-0 mt-3 flex list-decimal flex-col gap-4 pl-5 text-[14.5px] leading-relaxed">
        {questions.map((q, i) => {
          const a = quizAnswer(q);
          return (
            <li key={i} className="flex flex-col gap-1.5">
              <span>
                <span className="text-[12px] font-bold uppercase tracking-wide text-soft">{a.kind}</span>{" "}
                <span className="text-ink">{q.prompt}</span>
              </span>
              {a.code && <pre className="m-0 overflow-x-auto rounded-md bg-code-bg px-3 py-2 font-mono text-[13px] text-code-ink">{a.code}</pre>}
              {a.options && a.kind !== "Put in order" && (
                <span className="text-muted">Choices: {a.options.map((o, j) => <span key={j}>{j > 0 && " · "}<code>{o}</code></span>)}</span>
              )}
              <span>
                <strong>Answer:</strong>{" "}
                {a.answer.includes("\n") ? <pre className="m-0 mt-1 overflow-x-auto rounded-md bg-code-bg px-3 py-2 font-mono text-[13px] text-code-ink">{a.answer}</pre> : <code>{a.answer}</code>}
              </span>
              <span className="text-muted">{a.explain}</span>
            </li>
          );
        })}
      </ol>
    </details>
  );
}
