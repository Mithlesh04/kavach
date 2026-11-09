// The shape of one chapter's content. Chapters are data; the page shell,
// tabs and every widget are shared components — fix once, fixed everywhere.

import type { ReactNode } from "react";

export type HoverNote = {
  /** The exact code substring the note attaches to (first occurrence). */
  target: string;
  /** Category chip, e.g. "NEW CONCEPT". */
  chip: string;
  /** Chip color family. */
  kind: "new" | "why" | "style";
  what: string;
  why: string;
};

export type QuizQuestion =
  | {
      type: "spot";
      prompt: ReactNode;
      /** The code whose lines are clickable. */
      code: string;
      /** 1-based line that holds the bug. */
      answer: number;
      explain: string;
    }
  | {
      type: "order";
      prompt: ReactNode;
      /** The lines in their correct order; shown shuffled. */
      lines: string[];
      explain: string;
    }
  | {
      type: "bool";
      prompt: ReactNode;
      code?: string;
      answer: boolean;
      explain: string;
    }
  | {
      type: "choice";
      prompt: ReactNode;
      code?: string;
      options: string[];
      /** Index into options. */
      answer: number;
      explain: string;
    }
  | {
      type: "type-output";
      prompt: ReactNode;
      code: string;
      answer: string;
      explain: string;
    };

/**
 * A one-idea page (decision 0047): one scroll, in this order — the body,
 * "Try it yourself", "What Kavach does for you" (from content/smart.tsx),
 * "Go deeper", a short quiz.
 */
export type LessonDoc = {
  /** One sentence: the page's summary, its meta description and its llms.txt line. */
  lead: string;
  /** The idea in everyday words, for someone who has never programmed: no jargon, often a comparison from daily life. Shown under the lead. */
  simple?: ReactNode;
  minutes: number;
  /** Course keys of the pages to read first. */
  before?: string[];
  body: ReactNode;
  tryIt?: { task: ReactNode; answer: ReactNode };
  deeper?: ReactNode;
  quiz?: QuizQuestion[];
};

