// The Guides' pages, by course key: each one task from start to finish,
// with checked samples. Spread into the lessons of index.ts, as
// index-stdlib.ts is.

import type { LessonDoc } from "@/lib/chapter";
import { guideCliDoc } from "./guides-cli";
import { guideHttpDoc } from "./guides-http";
import { guideDataDoc } from "./guides-data";
import { guideAgentDoc } from "./guides-agent";
import { guideTestDoc } from "./guides-test";
import { guideTensorsDoc } from "./guides-tensors";
import { guideCoresDoc } from "./guides-cores";
import { guideCDoc } from "./guides-c";

export const guideLessons: Record<string, LessonDoc> = {
  "guides/build-a-command-line-tool": guideCliDoc,
  "guides/write-an-http-service": guideHttpDoc,
  "guides/read-and-write-data-files": guideDataDoc,
  "guides/build-an-agent-with-limits-and-approvals": guideAgentDoc,
  "guides/test-with-contracts-and-generated-tests": guideTestDoc,
  "guides/data-work-with-tensors": guideTensorsDoc,
  "guides/use-every-core-and-the-gpu": guideCoresDoc,
  "guides/call-a-c-library-safely": guideCDoc,
};
