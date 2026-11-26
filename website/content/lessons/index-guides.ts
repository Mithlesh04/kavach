// The Guides' pages, by course key: each one task from start to finish,
// with checked samples. Spread into the lessons of index.ts, as
// index-stdlib.ts is.

import type { LessonDoc } from "@/lib/chapter";
import { guideCliDoc } from "./guides-cli";

export const guideLessons: Record<string, LessonDoc> = {
  "guides/build-a-command-line-tool": guideCliDoc,
};
