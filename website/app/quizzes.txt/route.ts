import { lessons } from "@/content/lessons";
import { quizAnswer } from "@/lib/quiz-text";
import { SITE_URL, visibleNodes } from "@/lib/site";

export const dynamic = "force-static";

// quizzes.txt: every page's quiz with its answers, as Markdown, for AI
// tools that teach Kavach or check a learner's answer. llms.txt links it.
export function GET() {
  const lines: string[] = [
    "# Kavach quizzes, with answers",
    "",
    "> Each page of the Kavach course ends with a short \"check yourself\" quiz.",
    "> Here is every question, its correct answer, and why. The code is Kavach;",
    "> every answer was written against the page it belongs to.",
    "",
  ];
  let part = "";
  for (const node of visibleNodes()) {
    const quiz = lessons[node.key]?.quiz;
    if (!quiz?.length) continue;
    if (node.part.title !== part) {
      part = node.part.title;
      lines.push(`## Part ${node.part.n}: ${part}`, "");
    }
    const title = node.sub ? `${node.page.title}: ${node.title}` : node.title;
    lines.push(`### [${title}](${SITE_URL}${node.href})`, "");
    quiz.map(quizAnswer).forEach((a, i) => {
      lines.push(`${i + 1}. ${a.kind}: ${a.question}`, "");
      if (a.code) lines.push("```kavach", a.code, "```", "");
      if (a.options && a.kind === "Put in order") lines.push("   Lines, shuffled on the page:", "", "```kavach", ...[...a.options].sort(), "```", "");
      else if (a.options) lines.push(`   Choices: ${a.options.map((o) => `\`${o}\``).join(", ")}`, "");
      if (a.answer.includes("\n")) lines.push("   Answer:", "", "```kavach", a.answer, "```", "");
      else lines.push(`   Answer: \`${a.answer}\``, "");
      lines.push(`   Why: ${a.explain}`, "");
    });
  }
  return new Response(lines.join("\n"), {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}
