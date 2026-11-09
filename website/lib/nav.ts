// Addresses in the course tree (lib/course.ts, decision 0047): every page
// and sub-page is a node with a key ("basics/for-loop/over-a-range") and an
// href. The tree itself lives in course.ts; which nodes have content is
// decided in lib/site.ts.

import { course, type CoursePage, type CoursePart, type CourseSub } from "./course";

export type Level = "beginner" | "intermediate" | "advanced";

export const levelLabels: Record<Level, string> = {
  beginner: "Beginner",
  intermediate: "Intermediate",
  advanced: "Advanced",
};

export type CourseNode = {
  key: string;
  href: string;
  title: string;
  part: CoursePart;
  page: CoursePage;
  sub?: CourseSub;
};

/** Every page and sub-page, in reading order: a page, then its sub-pages. */
export function allNodes(): CourseNode[] {
  return course.flatMap((part) =>
    part.pages.flatMap((page) => {
      const pageKey = `${part.slug}/${page.slug}`;
      const own: CourseNode = { key: pageKey, href: `/docs/${pageKey}/`, title: page.title, part, page };
      return [
        own,
        ...page.subs.map((sub) => ({
          key: `${pageKey}/${sub.slug}`,
          href: `/docs/${pageKey}/${sub.slug}/`,
          title: sub.title,
          part,
          page,
          sub,
        })),
      ];
    }),
  );
}

export function findNode(key: string): CourseNode | undefined {
  return allNodes().find((n) => n.key === key);
}
