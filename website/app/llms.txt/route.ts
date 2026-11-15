import { lessons } from "@/content/lessons";
import { winsAgainstCpp } from "@/components/ratio";
import { SITE_URL, visibleNodes } from "@/lib/site";
import { course } from "@/lib/course";

export const dynamic = "force-static";

// llms.txt: the docs, mapped for AI tools (the language's own spec
// champions this; the site practices it). One line per page with its
// lead, grouped by part of the course.
export function GET() {
  const { wins, total } = winsAgainstCpp();
  const lines: string[] = [
    "# Kavach",
    "",
    "> A compiled, statically typed, general-purpose programming language:",
    "> as quick to try as a script (`kavach run`), compiled to one native",
    "> executable (`kavach build`), with no garbage collector. Every integer",
    `> and index checked, and faster than C++ in ${wins} of ${total} published,`,
    "> reproducible benchmarks.",
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
  lines.push("", "## Source", "", "- [Repository](https://github.com/Mithlesh04/kavach)", "");
  return new Response(lines.join("\n"), {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}
