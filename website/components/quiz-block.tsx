import { codeLines } from "./code-block";
import { Quiz, type QuizCode } from "./quiz";
import type { QuizQuestion } from "@/lib/chapter";

/**
 * The quiz with its code colored by the repository's grammar. The quiz runs
 * in the browser, where the grammar isn't loaded, so each question's code is
 * colored here while the site builds, one HTML string per line.
 */
export async function QuizBlock({ questions, nextTitle }: { questions: QuizQuestion[]; nextTitle?: string }) {
  const colored: (QuizQuestion & QuizCode)[] = await Promise.all(
    questions.map(async (q) => {
      if (q.type === "order") return { ...q, linesHtml: await Promise.all(q.lines.map(async (l) => (await codeLines(l))[0] ?? "")) };
      if ("code" in q && q.code) return { ...q, codeHtml: await codeLines(q.code) };
      return q;
    }),
  );
  return <Quiz questions={colored} nextTitle={nextTitle} />;
}
