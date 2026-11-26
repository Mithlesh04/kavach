import { C } from "@/components/inline-code";
import { PathFollow } from "@/components/path-follow";
import type { LessonDoc } from "@/lib/chapter";
import { course } from "@/lib/course";
import { findNode } from "@/lib/nav";
import { pathNodes, paths, type Step } from "@/lib/paths";
import { lessons } from "./index";
import { H2, P } from "./parts";

// ---------- Start here › Learning Paths ----------
//
// The six routes of the design canvas (Kavach Docs Concept, "Learning
// paths"): each an ordered list of pages that exist. Following one keeps
// it in this browser, and every page's path bar then shows the step and
// the next page on it. Read pages get a tick in the sidebar, whatever the
// path.

/** What a step is called: a part's title, or a page's. */
function label(step: Step): string {
  if ("part" in step) return course.find((p) => p.slug === step.part)?.title ?? step.part;
  return findNode(step.page)?.title ?? step.page;
}

/**
 * The six paths, as cards. A component, so the paths' pages are worked out
 * when the page renders: the course's page list imports the lessons, and
 * this file is one of them.
 */
function PathCards() {
  return (
    <div className="grid gap-4 [grid-template-columns:repeat(auto-fit,minmax(min(100%,460px),1fr))]">
      {paths.map((p) => {
        const nodes = pathNodes(p);
        // The pages' own reading times, added up.
        const minutes = nodes.reduce((sum, n) => sum + (lessons[n.key]?.minutes ?? 0), 0);
        const time = p.time ?? (minutes < 90 ? `about ${minutes} min` : `about ${Math.round(minutes / 60)} h`);
        return (
          <section key={p.id} className="flex flex-col gap-3 rounded-[14px] border border-line bg-panel p-5">
            <div className="flex items-start gap-3">
              <div className="min-w-0 flex-1">
                <h2 className="m-0 text-[19px] font-extrabold tracking-tight text-ink">{p.title}</h2>
                <p className="m-0 mt-1 text-sm text-muted">{p.who}</p>
              </div>
              <span className="shrink-0 text-xs text-soft">{time}</span>
            </div>
            <ol className="m-0 flex list-none flex-wrap items-center gap-1.5 p-0 text-[13px]">
              {p.steps.map((s, i) => (
                <li key={i} className="flex items-center gap-1.5">
                  <span className="rounded-md border border-line bg-face px-2 py-1 text-ink">{label(s)}</span>
                  {i < p.steps.length - 1 && <span aria-hidden="true" className="text-soft">→</span>}
                </li>
              ))}
            </ol>
            <p className="m-0 border-t border-line-2 pt-3 text-xs text-muted">
              <strong className="text-ink">Read:</strong> {p.read}. <strong className="text-ink">Ends with:</strong> {p.ends}.{" "}
              {nodes.length} pages.
            </p>
            <div>
              <PathFollow id={p.id} first={nodes[0].href} />
            </div>
          </section>
        );
      })}
    </div>
  );
}

export const pathsDoc: LessonDoc = {
  lead: "Six routes through one course, for six kinds of reader. A path is an ordered list of pages that already exist. Follow one, and every page shows your step and the next page on your path.",
  simple: <>Not everyone needs every page. Pick the route that fits what you already know, and the site walks you through the pages that matter for you, in a good order.</>,
  minutes: 2,
  body: (
    <>
      <PathCards />
      <H2>How paths work</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Following a path keeps it in this browser only: no account, and nothing is sent anywhere.</li>
        <li>Each page then shows <strong className="text-ink">Your path</strong> above its Previous and Next links: the step you are on, and the next page on your path.</li>
        <li>A page you read gets a tick in the sidebar. Ticks belong to pages, not paths, so changing paths keeps every one.</li>
        <li>Off your path, the bar takes you back to the first page on it you haven&apos;t read.</li>
        <li>Every page starts with <C>Before this:</C> links, so a reader who skips ahead sees which page they missed.</li>
      </ul>
      <P>Leave a path at any time from the bar, or here.</P>
    </>
  ),
};
