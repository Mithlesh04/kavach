// The Concepts' pages, by course key: each one design choice, the why in
// depth, as its decision record makes it. Spread into the lessons of
// index.ts, as index-stdlib.ts is.

import type { LessonDoc } from "@/lib/chapter";
import { conceptCapabilitiesDoc, conceptErrorsDoc, conceptMemoryDoc, conceptValuesDoc } from "./concepts-core";
import { conceptNumbersDoc, conceptOneCoreDoc, conceptTestsDoc } from "./concepts-checking";
import { conceptAgentsDoc, conceptAiDoc, conceptSmallFilesDoc } from "./concepts-scale";

export const conceptLessons: Record<string, LessonDoc> = {
  "concepts/values-not-shared-references": conceptValuesDoc,
  "concepts/errors-as-values": conceptErrorsDoc,
  "concepts/capabilities-in-types": conceptCapabilitiesDoc,
  "concepts/memory-without-a-garbage-collector": conceptMemoryDoc,
  "concepts/tests-from-contracts": conceptTestsDoc,
  "concepts/the-one-core-answer": conceptOneCoreDoc,
  "concepts/no-silent-number-surprises": conceptNumbersDoc,
  "concepts/small-files-enforced": conceptSmallFilesDoc,
  "concepts/built-for-ai-to-read-and-write": conceptAiDoc,
  "concepts/agents-with-limits": conceptAgentsDoc,
};
