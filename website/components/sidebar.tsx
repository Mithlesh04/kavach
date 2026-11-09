"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { useNavDrawer } from "@/components/nav-drawer";
import type { SidePart } from "@/lib/site";

/**
 * The course tree (decision 0047): parts that fold open, one page per
 * idea, and a ▾ on a page that has sub-pages. The part and the page you
 * are on open by themselves; clicking the title of the page you are on
 * folds or unfolds its sub-pages. Sticky: the body scrolls, this pane stays
 * put with its own scrollbar. Quiet neutral colors; only where you are
 * carries weight. Below Tailwind's `lg` it is a menu that slides in over the
 * page, with the site's main links on top, so a phone shows the page first.
 */
export function Sidebar({ tree, links }: { tree: SidePart[]; links: { title: string; href: string }[] }) {
  const pathname = usePathname() ?? "";
  const on = (href: string) => pathname === href || pathname === href.replace(/\/$/, "");
  const slashed = (p: string) => (p.endsWith("/") ? p : `${p}/`);
  const inside = (href: string) => slashed(pathname).startsWith(slashed(href));
  const [openParts, setOpenParts] = useState<Record<string, boolean>>({});
  const [openPages, setOpenPages] = useState<Record<string, boolean>>({});
  const { open: menuOpen, close } = useNavDrawer();
  const closeButton = useRef<HTMLButtonElement>(null);
  const pane = useRef<HTMLElement>(null);

  // An opened menu shows where you are, and the keyboard starts in it.
  useEffect(() => {
    if (!menuOpen) return;
    closeButton.current?.focus({ preventScroll: true });
    pane.current?.querySelector('[aria-current="page"]')?.scrollIntoView({ block: "center" });
  }, [menuOpen]);

  return (
    <>
    {menuOpen && <button aria-label="Close the course menu" tabIndex={-1} onClick={close} className="fixed inset-0 z-40 cursor-default border-0 bg-scrim lg:hidden" />}
    <nav
      ref={pane}
      id="course-nav"
      aria-label="Kavach tutorial"
      className={`box-border overflow-y-auto border-r border-line-2 py-6 pl-5 pr-3 lg:sticky lg:top-0 lg:max-h-screen lg:w-[280px] lg:shrink-0 max-lg:fixed max-lg:inset-y-0 max-lg:left-0 max-lg:z-50 max-lg:w-[min(340px,86vw)] max-lg:bg-bg max-lg:pt-3 max-lg:duration-200 ${menuOpen ? "max-lg:transition-transform" : "max-lg:invisible max-lg:-translate-x-full max-lg:transition-[transform,visibility]"}`}
    >
      {/* Stays on top: the menu opens scrolled to the current page, and must still be easy to close. */}
      <div className="sticky -top-3 z-10 mb-3 flex items-center justify-between bg-bg py-1 lg:hidden">
        <div className="flex flex-wrap gap-x-4 gap-y-1 px-2.5 text-sm font-semibold">
          {links.map((l) => (
            <a key={l.href} href={l.href} className="py-1 text-muted no-underline">
              {l.title}
            </a>
          ))}
        </div>
        <button
          ref={closeButton}
          onClick={close}
          aria-label="Close the course menu"
          className="flex h-10 w-10 shrink-0 cursor-pointer items-center justify-center rounded-lg border-0 bg-transparent text-muted"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="m6 6 12 12M18 6 6 18" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          </svg>
        </button>
      </div>
      <p className="m-0 mb-2 px-2.5 text-[11px] font-bold uppercase tracking-[0.12em] text-side-head">Kavach tutorial</p>
      {tree.map((part) => {
        const here = part.pages.some((p) => inside(p.href));
        const open = openParts[part.slug] ?? here;
        return (
          <div key={part.slug} className="border-t border-line-2 first-of-type:border-t-0">
            <button
              aria-expanded={open}
              onClick={() => setOpenParts({ ...openParts, [part.slug]: !open })}
              className="flex w-full cursor-pointer items-center gap-2 border-0 bg-transparent px-2.5 py-2.5 text-left"
            >
              <span className="w-5 font-mono text-[11px] text-side-head">{part.n}</span>
              <span className={`flex-1 text-[13px] font-bold ${open ? "text-ink" : "text-side-link"}`}>{part.title}</span>
              <span aria-hidden="true" className="text-[10px] text-side-head">{open ? "▴" : "▾"}</span>
            </button>
            {open && (
              <ul className="m-0 mb-2 list-none p-0">
                {part.pages.map((page) => {
                  const current = on(page.href);
                  const subsOpen = page.subs.length > 0 && (openPages[page.key] ?? inside(page.href));
                  return (
                    <li key={page.key}>
                      <div className="flex items-center">
                        <Link
                          href={page.href}
                          aria-current={current ? "page" : undefined}
                          onClick={(e) => {
                            if (page.subs.length === 0) return;
                            // On the page already: its title folds the sub-pages, as the ▾ does.
                            // Elsewhere: go there, with its sub-pages shown.
                            if (current) e.preventDefault();
                            setOpenPages({ ...openPages, [page.key]: current ? !subsOpen : true });
                          }}
                          className={
                            current
                              ? "flex-1 rounded-md bg-side-active py-[5px] pl-9 pr-2 text-[13.5px] font-semibold leading-relaxed text-ink no-underline"
                              : "flex-1 rounded-md py-[5px] pl-9 pr-2 text-[13.5px] leading-relaxed text-side-link no-underline transition-colors duration-150 hover:text-ink"
                          }
                        >
                          {page.title}
                        </Link>
                        {page.subs.length > 0 && (
                          <button
                            aria-label={`${subsOpen ? "Hide" : "Show"} the ${page.title} sub-pages`}
                            aria-expanded={subsOpen}
                            onClick={() => setOpenPages({ ...openPages, [page.key]: !subsOpen })}
                            className="cursor-pointer border-0 bg-transparent px-2 py-1 text-[10px] text-side-head hover:text-ink"
                          >
                            {subsOpen ? "▴" : "▾"}
                          </button>
                        )}
                      </div>
                      {subsOpen && (
                        <ul className="m-0 list-none p-0">
                          {page.subs.map((sub) => {
                            const subHere = on(sub.href);
                            return (
                              <li key={sub.key}>
                                <Link
                                  href={sub.href}
                                  aria-current={subHere ? "page" : undefined}
                                  className={
                                    subHere
                                      ? "block rounded-md bg-side-active py-1 pl-12 pr-2 text-[13px] font-semibold text-ink no-underline"
                                      : "block rounded-md py-1 pl-12 pr-2 text-[13px] text-side-link no-underline hover:text-ink"
                                  }
                                >
                                  {sub.title}
                                </Link>
                              </li>
                            );
                          })}
                        </ul>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        );
      })}
    </nav>
    </>
  );
}
