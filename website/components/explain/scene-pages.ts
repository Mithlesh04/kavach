// Which scenes a page shows, read from the page's own content: every
// <Explainer id> and <MemorySteps name> in its body or "Go deeper". So the
// page is the one place that says where a scene lives, and /explainers.txt
// and the page's schema.org data follow it.

import { isValidElement, type ReactNode } from "react";
import type { LessonDoc } from "@/lib/chapter";
import { MemorySteps } from "../memory-steps";
import { Explainer } from "./explainer";
import { memoryId, sceneInfo } from "./scenes";
import type { SceneInfo } from "./types";

function walk(node: ReactNode, out: string[]) {
  if (Array.isArray(node)) {
    for (const n of node) walk(n, out);
    return;
  }
  if (!isValidElement<{ children?: ReactNode; id?: string; name?: string }>(node)) return;
  if (node.type === Explainer && node.props.id) out.push(node.props.id);
  if (node.type === MemorySteps && node.props.name) out.push(memoryId(node.props.name));
  walk(node.props.children, out);
}

/** The scenes on a page, in the order they appear. */
export function pageScenes(doc: LessonDoc | undefined): SceneInfo[] {
  if (!doc) return [];
  const ids: string[] = [];
  walk(doc.body, ids);
  walk(doc.deeper, ids);
  return ids.map(sceneInfo);
}
