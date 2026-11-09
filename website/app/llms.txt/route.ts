import { lessons } from "@/content/lessons";
import { winsAgainstCpp } from "@/components/ratio";
import { SITE_URL, visibleNodes } from "@/lib/site";

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
    "The course teaches one idea per page, in 14 parts. A page with several",
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
