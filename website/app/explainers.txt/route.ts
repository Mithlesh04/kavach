import { lessons } from "@/content/lessons";
import { pageScenes } from "@/components/explain/scene-pages";
import { sceneMarkdown } from "@/components/explain/scene-text";
import { SCENES } from "@/components/explain/scenes";
import { SITE_URL, visibleNodes } from "@/lib/site";

export const dynamic = "force-static";

// explainers.txt: every step-by-step animation on the site as Markdown,
// for AI tools and readers that don't run them: each scene's caveman
// summary, its checked code and output, and every step in words. A scene
// lives on the page that shows it; llms.txt links this file.
export function GET() {
  const lines: string[] = [
    "# Kavach explainers, step by step",
    "",
    "> The Kavach course shows how code runs on the machine as animations:",
    "> the CPU, RAM, the stack, the heap, the OS. Here is each one as text:",
    "> a very short \"caveman\" summary, the code (a sample checked against a",
    "> real run), and every step with the line it runs and what happens.",
    "",
  ];
  const shown = new Set<string>();
  let part = "";
  for (const node of visibleNodes()) {
    const scenes = pageScenes(lessons[node.key]);
    if (!scenes.length) continue;
    if (node.part.title !== part) {
      part = node.part.title;
      lines.push(`## Part ${node.part.n}: ${part}`, "");
    }
    for (const info of scenes) {
      shown.add(info.id);
      lines.push(...sceneMarkdown(info, `${SITE_URL}${node.href}`));
    }
  }
  const missing = Object.keys(SCENES).filter((id) => !shown.has(id));
  if (missing.length) throw new Error(`explainers.txt: no page shows the scenes ${missing.join(", ")}; show them or remove them (components/explain/scenes.ts)`);
  return new Response(lines.join("\n"), {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}
