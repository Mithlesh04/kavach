import type { CourseNode } from "./nav";
import { visibleNodes } from "./site";

// Learning paths: six routes through the one course, from the design canvas
// (Kavach Docs Concept, "Learning paths", 2026-11-03). A path is an ordered
// list of pages that already exist, never new copies. A step is a whole
// part (its pages and sub-pages in course order) or one page (with its
// sub-pages). A step naming nothing that exists fails the build.

export type Step = { part: string } | { page: string };

export type LearningPath = {
  id: string;
  title: string;
  who: string;
  /** How long it takes, when it isn't the sum of its pages' reading times. */
  time?: string;
  /** Which layers of each page to read. */
  read: string;
  /** Where the path ends. */
  ends: string;
  steps: Step[];
};

export const paths: LearningPath[] = [
  {
    id: "new",
    title: "I have never programmed",
    who: "Starts from what a program is. Small steps, a success on every page.",
    read: "the page, Try it yourself, the quiz",
    ends: "a data file read and written, then choose the next path",
    steps: [{ part: "start" }, { part: "basics" }, { part: "functions" }, { part: "collections" }, { part: "types" }, { part: "errors" }, { page: "standard-library/files" }, { page: "standard-library/json" }, { page: "guides/read-and-write-data-files" }],
  },
  {
    id: "python-js",
    title: "I know Python or JavaScript",
    who: "Already comfortable with code; new to static types and errors as values.",
    read: "the page, Go deeper, the quiz",
    ends: "the refund agent",
    steps: [{ page: "start/coming-from-another-language/python" }, { page: "start/coming-from-another-language/javascript" }, { page: "start/kavach-home" }, { part: "types" }, { part: "errors" }, { part: "standard-library" }, { part: "agents" }, { page: "guides/build-an-agent-with-limits-and-approvals" }],
  },
  {
    id: "systems",
    title: "I know C++, Rust or Go",
    who: "Wants the memory model, the speed claims and the proof behind them.",
    read: "Go deeper, the benchmarks",
    ends: "a program on every core and the GPU",
    steps: [{ page: "start/coming-from-another-language/cpp" }, { page: "start/coming-from-another-language/rust" }, { page: "start/coming-from-another-language/go" }, { page: "start/kavach-home" }, { part: "errors" }, { part: "memory" }, { part: "concurrency" }, { part: "speed" }, { part: "other-languages" }, { page: "guides/use-every-core-and-the-gpu" }],
  },
  {
    id: "agents",
    title: "I build AI agents",
    who: "Wants a model in the loop, with limits, approvals and an audit trail.",
    read: "the page, Go deeper, the quiz",
    ends: "the refund agent",
    steps: [{ page: "start/kavach-home" }, { page: "types/records" }, { page: "types/unions" }, { part: "errors" }, { part: "agents" }, { page: "guides/build-an-agent-with-limits-and-approvals" }],
  },
  {
    id: "services",
    title: "I build services and back ends",
    who: "Wants HTTP, gRPC, JSON and tasks, shipped as one fast executable.",
    read: "the page, the benchmarks",
    ends: "a JSON HTTP service",
    steps: [{ page: "start/kavach-home" }, { page: "standard-library/json" }, { part: "networks" }, { part: "concurrency" }, { page: "errors/testing" }, { page: "speed/native-executables" }, { page: "guides/write-an-http-service" }],
  },
  {
    id: "answer",
    title: "I just need an answer",
    who: "Knows Kavach; has one question right now.",
    time: "a minute",
    read: "the Reference",
    ends: "back to work",
    steps: [{ part: "reference" }],
  },
];

/** A path's pages, in order: each step's nodes, each page once. */
export function pathNodes(path: LearningPath): CourseNode[] {
  const all = visibleNodes();
  const out: CourseNode[] = [];
  const seen = new Set<string>();
  for (const step of path.steps) {
    const nodes =
      "part" in step
        ? all.filter((n) => n.part.slug === step.part)
        : all.filter((n) => n.key === step.page || n.key.startsWith(`${step.page}/`));
    const where = "part" in step ? `part ${step.part}` : `page ${step.page}`;
    if (nodes.length === 0) throw new Error(`learning path ${path.id}: ${where} has no pages (lib/paths.ts)`);
    for (const n of nodes) {
      if (!seen.has(n.key)) {
        seen.add(n.key);
        out.push(n);
      }
    }
  }
  return out;
}
