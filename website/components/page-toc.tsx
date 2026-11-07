"use client";

import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";

type Item = { id: string; text: string };

const slug = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");

/** The headings a reader can see now, each with an id unique on the page: closed "Go deeper" parts are left out. */
function headings(): { items: Item[]; found: HTMLElement[] } {
  const found = [...document.querySelectorAll<HTMLElement>("main h2")].filter((h) => !h.closest("details:not([open])"));
  const seen = new Set<string>();
  const items = found.map((h) => {
    const text = h.textContent?.trim() ?? "";
    let id = h.id || slug(text);
    for (let n = 2; seen.has(id); n++) id = `${h.id || slug(text)}-${n}`;
    seen.add(id);
    h.id = id;
    return { id, text };
  });
  return { items, found };
}

/**
 * "On this page", in the empty space a wide screen leaves beside the text. Read
 * from the page's headings, so no page lists them twice; a page with one
 * heading or none shows nothing. Narrower screens don't have the room.
 */
export function PageToc() {
  const pathname = usePathname();
  const [items, setItems] = useState<Item[]>([]);
  const [active, setActive] = useState("");

  useEffect(() => {
    let observer: IntersectionObserver | undefined;
    const frame = requestAnimationFrame(() => {
      const { items: now, found } = headings();
      setItems(now);
      setActive(now[0]?.id ?? "");
      // The last heading to cross the top quarter of the screen is the part being read.
      observer = new IntersectionObserver(
        () => {
          const passed = found.filter((h) => h.getBoundingClientRect().top < window.innerHeight * 0.25);
          setActive((passed.at(-1) ?? found[0])?.id ?? "");
        },
        { rootMargin: "0px 0px -75% 0px", threshold: [0, 1] },
      );
      found.forEach((h) => observer?.observe(h));
    });
    return () => {
      cancelAnimationFrame(frame);
      observer?.disconnect();
    };
  }, [pathname]);

  if (items.length < 2) return <div aria-hidden="true" className="hidden w-[232px] shrink-0 min-[1440px]:block" />;
  return (
    <nav
      aria-label="On this page"
      className="sticky top-0 box-border hidden max-h-screen w-[232px] shrink-0 overflow-y-auto py-8 pl-5 pr-3 min-[1440px]:block"
    >
      <p className="m-0 mb-2.5 text-[11px] font-bold uppercase tracking-[0.12em] text-side-head">On this page</p>
      <ul className="m-0 list-none border-l border-line-2 p-0">
        {items.map((item) => (
          <li key={item.id}>
            <a
              href={`#${item.id}`}
              aria-current={item.id === active ? "location" : undefined}
              className={
                item.id === active
                  ? "-ml-px block border-l-2 border-accent py-1 pl-3 text-[13px] font-semibold leading-snug text-ink no-underline"
                  : "-ml-px block border-l-2 border-transparent py-1 pl-3 text-[13px] leading-snug text-side-link no-underline hover:text-ink"
              }
            >
              {item.text}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
