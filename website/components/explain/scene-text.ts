// A scene as text, for what reads a page without running it: the "Every
// step, as text" list under each animation, /explainers.txt (linked from
// /llms.txt) and the page's schema.org data. Like lib/quiz-text.ts for
// quizzes. It also loads a scene's code and checks each step against it.

import { loadSample, sampleBody } from "@/lib/samples";
import type { SceneInfo } from "./types";

/**
 * The code a scene shows, and what its sample prints. Each step must name
 * a line that is there, so a changed sample fails the build instead of
 * showing a wrong picture.
 */
export function sceneCode(info: SceneInfo): { code: string; lines: string[]; output: string } {
  const sample = info.code === "main" ? sampleBody(info.sample) : loadSample(info.sample);
  const lines = sample.code.split("\n");
  for (const s of info.steps) {
    if (!lines[s.line]?.includes(s.at)) {
      throw new Error(`${info.sample}: scene "${info.id}" expects "${s.at}" on line ${s.line + 1}, which is "${lines[s.line] ?? ""}" (components/explain/)`);
    }
  }
  return { code: sample.code, lines, output: sample.output ?? "" };
}

export type StepText = { n: number; line: number; code: string; what: string };

/** Each step: its number, the line it runs (from 1), that line's code, and what happens. */
export function stepsText(info: SceneInfo, lines: string[]): StepText[] {
  return info.steps.map((s, i) => ({ n: i + 1, line: s.line + 1, code: lines[s.line].trim(), what: s.caption }));
}

/** One scene as Markdown: its title, the caveman line, the code, what it prints, and every step. */
export function sceneMarkdown(info: SceneInfo, url: string): string[] {
  const { code, lines, output } = sceneCode(info);
  return [
    `### [${info.title}](${url}#scene-${info.id})`,
    "",
    `Caveman says: ${info.caveman}`,
    "",
    "```kavach",
    code,
    "```",
    "",
    ...(output ? ["It prints, checked against a real run:", "", "```", output, "```", ""] : []),
    ...stepsText(info, lines).map((s) => `${s.n}. Line ${s.line}, \`${s.code}\`: ${s.what}`),
    "",
  ];
}

/** schema.org data for one scene: a learning resource that is an animation, with its steps as its text. */
export function sceneStructured(info: SceneInfo, pageUrl: string) {
  const { lines } = sceneCode(info);
  return {
    "@context": "https://schema.org",
    "@type": "LearningResource",
    name: info.title,
    url: `${pageUrl}#scene-${info.id}`,
    learningResourceType: "Step-by-step animation",
    interactivityType: "mixed",
    inLanguage: "en",
    abstract: info.caveman,
    about: { "@type": "ComputerLanguage", name: "Kavach" },
    isPartOf: { "@type": "TechArticle", url: pageUrl },
    text: stepsText(info, lines)
      .map((s) => `Step ${s.n}. Line ${s.line}, ${s.code}: ${s.what}`)
      .join("\n"),
  };
}
