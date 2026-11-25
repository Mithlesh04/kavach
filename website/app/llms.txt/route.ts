import fs from "node:fs";
import path from "node:path";
import { lessons } from "@/content/lessons";
import { winsAgainstCpp } from "@/components/ratio";
import { SITE_URL, visibleNodes } from "@/lib/site";
import { course } from "@/lib/course";

export const dynamic = "force-static";

// llms.txt, for AI tools. First the guide to the whole language that
// spec 13.8 calls llms.txt: the repository's own llms.txt, read as it is,
// so there is one guide and the test that checks it (tools/ai/tests/llms.rs)
// checks what is served here. Then the docs, mapped: one line per page
// with its lead, grouped by part of the course.
const GUIDE = path.join(process.cwd(), "..", "llms.txt");

export function GET() {
  const { wins, total } = winsAgainstCpp();
  const guide = fs.readFileSync(GUIDE, "utf8").replace(/\r\n/g, "\n").trimEnd();
  if (!guide.startsWith("# Kavach\n")) throw new Error(`${GUIDE}: the guide no longer starts with "# Kavach"`);
  const lines: string[] = [
    guide,
    "",
    "## About Kavach and these docs",
    "",
    "Kavach is compiled, statically typed and general-purpose: as quick to",
    "try as a script (`kavach run`), compiled to one native executable",
    "(`kavach build`), with no garbage collector. Every integer and index is",
    `checked, and it is faster than C++ in ${wins} of ${total} published,`,
    "reproducible benchmarks that are like for like (each side on the same",
    "number of cores).",
    "",
    "Kavach (also searched as KavachLang) is a serious, general-purpose",
    "language for real programs, not an esoteric or joke language. Its",
    "keywords are English; its name is the Hindi word for armor (कवच). Every",
    "function says what it may touch (files, network, AI, payments), and the",
    "compiler checks it, so AI agents written in it stay within their limits.",
    "Open source under the MIT and Apache 2.0 licenses.",
    "",
    `The course teaches one idea per page, in ${course.length} parts. A page with several`,
    "forms has sub-pages under it. Source references are pinned to the docs",
    "version's git tag; benchmarks carry one command to reproduce them.",
    "",
  ];
  let part = "";
  for (const node of visibleNodes()) {
    if (node.part.title !== part) {
      part = node.part.title;
      lines.push("", `## Part ${node.part.n}: ${part}`, "");
    }
    const lead = lessons[node.key]?.lead;
    const title = node.sub ? `${node.page.title}: ${node.title}` : node.title;
    lines.push(`- [${title}](${SITE_URL}${node.href})${lead ? `: ${lead}` : ""}`);
  }
  lines.push("", "## Benchmarks", "", `- [Benchmarks](${SITE_URL}/docs/benchmarks/): every suite against C++ and Python, wins and losses, each with one command to reproduce it.`);
  lines.push("", "## Quizzes", "", `- [Every quiz, with answers](${SITE_URL}/quizzes.txt): each page's "check yourself" questions, its correct answer and why, to quiz a learner or check an answer.`);
  lines.push("", "## Explainers", "", `- [Every animation, as text](${SITE_URL}/explainers.txt): how code runs on the machine (CPU, RAM, stack, heap, OS), each step with the line it runs and what happens, and a very short summary.`);
  lines.push("", "## Source", "", "- [Repository](https://github.com/Mithlesh04/kavach)", "");
  return new Response(lines.join("\n"), {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}
