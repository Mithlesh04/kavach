// The course as the site shows it: the tree (lib/course.ts) joined with the
// content that exists. A page appears in the sidebar, routes, sitemap and
// llms.txt only once it has content (decision 0047). Server-only: content
// modules read their samples from disk.

import fs from "node:fs";
import path from "node:path";
import { lessons } from "@/content/lessons";
import { smart } from "@/content/smart";
import { allNodes, findNode, type CourseNode } from "./nav";

// Where the site is served, for canonical links, the sitemap and llms.txt:
// GitHub Pages by default, or the address another host gives it (README).
export const SITE_URL = process.env.SITE_URL ?? "https://mithlesh04.github.io/kavach";

export function hasContent(key: string): boolean {
  return key in lessons;
}

// Build-time guards, run on first use (never at import: content modules
// import this file too): every piece of content has a place in the
// course, a sub-page never shows without its page, and every smart thing
// names a real page and a proof file that exists.
let checked = false;
function check() {
  if (checked) return;
  checked = true;
  for (const key of Object.keys(lessons)) {
    const node = findNode(key);
    if (!node) throw new Error(`content "${key}" has no place in lib/course.ts`);
    if (node.sub && !hasContent(`${node.part.slug}/${node.page.slug}`)) {
      throw new Error(`sub-page "${key}" has content but its page does not`);
    }
  }
  for (const thing of smart) {
    if (!findNode(thing.where)) throw new Error(`smart thing "${thing.title}": no course page "${thing.where}"`);
    if (!fs.existsSync(path.join(process.cwd(), "..", thing.proof.file))) {
      throw new Error(`smart thing "${thing.title}": its proof ${thing.proof.file} no longer exists`);
    }
  }
}

/** Every page and sub-page with content, in reading order. */
export function visibleNodes(): CourseNode[] {
  check();
  return allNodes().filter((n) => hasContent(n.key));
}

export function neighbors(key: string): { prev?: CourseNode; next?: CourseNode } {
  const all = visibleNodes();
  const i = all.findIndex((n) => n.key === key);
  return { prev: all[i - 1], next: all[i + 1] };
}

export type SideSub = { key: string; href: string; title: string };
export type SidePage = SideSub & { subs: SideSub[] };
export type SidePart = { n: number; slug: string; title: string; pages: SidePage[] };

/** The sidebar's data: parts and pages that have content. */
export function sidebarTree(): SidePart[] {
  const visible = visibleNodes();
  const parts: SidePart[] = [];
  for (const node of visible) {
    let part = parts.at(-1);
    if (part?.slug !== node.part.slug) {
      part = { n: node.part.n, slug: node.part.slug, title: node.part.title, pages: [] };
      parts.push(part);
    }
    const entry = { key: node.key, href: node.href, title: node.title };
    if (node.sub) part.pages.at(-1)?.subs.push(entry);
    else part.pages.push({ ...entry, subs: [] });
  }
  return parts;
}

/** The smart things a page teaches. */
export function smartFor(key: string) {
  return smart.filter((s) => s.where === key);
}
