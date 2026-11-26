"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { CHANGED, choosePath, chosenPath, loadPaths, markRead, readPages, type PathData } from "./path-store";

const slashed = (p: string) => (p.endsWith("/") ? p : `${p}/`);

/**
 * The reader's learning path, on every course page: which step this is and
 * the next page on the path, or the way back to the path from a page off
 * it. It also marks the page read. With no path chosen, it shows nothing:
 * the page's own Previous and Next links follow the course.
 */
export function PathBar() {
  const here = slashed(usePathname());
  const [path, setPath] = useState<PathData | null>(null);

  useEffect(() => {
    markRead(here);
    let live = true;
    const load = () => {
      const id = chosenPath();
      if (!id) return setPath(null);
      void loadPaths().then((all) => {
        if (live) setPath(all.find((p) => p.id === id) ?? null);
      });
    };
    load();
    window.addEventListener(CHANGED, load);
    return () => {
      live = false;
      window.removeEventListener(CHANGED, load);
    };
  }, [here]);

  if (!path) return null;
  const i = path.pages.findIndex((p) => p.href === here);
  const next = i >= 0 ? path.pages[i + 1] : undefined;
  // Off the path, the way back is its first page not read yet.
  const read = readPages();
  const resume = path.pages.find((p) => !read.has(p.href)) ?? path.pages[0];
  return (
    <aside aria-label="Your learning path" className="mx-10 mb-6 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-[10px] border border-line bg-face px-4 py-3 text-sm max-md:mx-5 max-sm:mx-4">
      <span className="font-bold text-ink">
        Your path: <Link href="/docs/start/learning-paths/" className="text-accent-text">{path.title}</Link>
      </span>
      {i >= 0 ? (
        <span className="text-soft">
          step {i + 1} of {path.pages.length}
        </span>
      ) : (
        <span className="text-soft">This page isn&apos;t on it.</span>
      )}
      <span className="flex-1" />
      {i >= 0 && next && (
        <Link href={next.href} className="rounded-lg bg-accent px-3 py-1.5 font-bold text-bg no-underline">
          Next on your path: {next.title} →
        </Link>
      )}
      {i >= 0 && !next && <span className="font-bold text-ink">The last step: well done.</span>}
      {i < 0 && resume && (
        <Link href={resume.href} className="font-bold text-accent-text">
          Back to your path: {resume.title}
        </Link>
      )}
      <button type="button" onClick={() => choosePath(null)} className="cursor-pointer border-0 bg-transparent p-0 text-soft underline">
        Leave the path
      </button>
    </aside>
  );
}
