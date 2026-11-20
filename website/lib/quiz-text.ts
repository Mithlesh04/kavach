// A page's quiz as plain questions and answers, for what reads the page
// without running it: search engines (schema.org Quiz data), AI tools
// (quizzes.txt) and readers who want every answer at once (QuizAnswers).
// The quiz itself runs in the browser and shows one question at a time.

import { isValidElement, type ReactNode } from "react";
import { C } from "@/components/inline-code";
import type { QuizQuestion } from "./chapter";

/** The text of a prompt; inline code keeps its backticks. */
export function nodeText(node: ReactNode): string {
  if (node === null || node === undefined || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeText).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) {
    const inner = nodeText(node.props.children);
    return node.type === C ? `\`${inner}\`` : inner;
  }
  return "";
}

export type QuizAnswer = {
  /** What the reader is asked to do, as the quiz names it. */
  kind: "Multiple choice" | "True or false" | "Find the bug" | "Put in order" | "Type the output";
  question: string;
  code?: string;
  /** The choices offered, when there are some. */
  options?: string[];
  answer: string;
  explain: string;
};

/** One question with its answer written out. */
export function quizAnswer(q: QuizQuestion): QuizAnswer {
  const question = nodeText(q.prompt);
  switch (q.type) {
    case "choice":
      return { kind: "Multiple choice", question, code: q.code, options: q.options, answer: q.options[q.answer], explain: q.explain };
    case "bool":
      return { kind: "True or false", question, code: q.code, options: ["True", "False"], answer: q.answer ? "True" : "False", explain: q.explain };
    case "spot": {
      const line = q.code.split("\n")[q.answer - 1] ?? "";
      return { kind: "Find the bug", question, code: q.code, answer: `Line ${q.answer}: ${line.trim()}`, explain: q.explain };
    }
    case "order":
      return { kind: "Put in order", question, options: q.lines, answer: q.lines.join("\n"), explain: q.explain };
    case "type-output":
      return { kind: "Type the output", question, code: q.code, answer: q.answer, explain: q.explain };
  }
}

/** schema.org Quiz data for a page's quiz: each question with its accepted answer. */
export function quizStructured(title: string, url: string, questions: QuizQuestion[]) {
  return {
    "@context": "https://schema.org",
    "@type": "Quiz",
    name: `${title}: check yourself`,
    url: `${url}#quiz`,
    about: { "@type": "Thing", name: title },
    educationalAlignment: { "@type": "AlignmentObject", alignmentType: "educationalSubject", targetName: "Kavach programming language" },
    hasPart: questions.map(quizAnswer).map((a) => ({
      "@type": "Question",
      eduQuestionType: a.options && a.kind !== "Put in order" ? "Multiple choice" : "Flashcard",
      text: a.code ? `${a.question}\n\n${a.code}` : a.question,
      acceptedAnswer: { "@type": "Answer", text: a.answer, answerExplanation: { "@type": "Comment", text: a.explain } },
      ...(a.options && a.kind !== "Put in order"
        ? { suggestedAnswer: a.options.filter((o) => o !== a.answer).map((o) => ({ "@type": "Answer", text: o })) }
        : {}),
    })),
  };
}
