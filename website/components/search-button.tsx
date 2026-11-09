"use client";

import { useEffect, useRef, useState } from "react";

/**
 * Search, fully static: Pagefind builds the index from the exported
 * site at build time (postbuild), and this loads it on demand in the
 * browser. Sub-results come from the per-tab headings, so a result
 * opens the right page. In `next dev` it loads the index
 * scripts/dev-search.mjs made from the last build.
 */

type SubResult = { title: string; url: string; excerpt: string };
type ResultData = {
  url: string;
  meta: { title?: string };
  excerpt: string;
  sub_results: SubResult[];
};
type Pagefind = {
  options: (o: { baseUrl: string }) => Promise<void>;
  init: () => void;
  search: (q: string) => Promise<{ results: { data: () => Promise<ResultData> }[] }>;
};

let pagefindPromise: Promise<Pagefind | null> | null = null;
function loadPagefind(): Promise<Pagefind | null> {
  pagefindPromise ??= (async () => {
    try {
      // A runtime URL import the bundler must leave alone.
      // `next dev` serves the index scripts/dev-search.mjs made from the last build.
      // Under a base path (GitHub Pages: /kavach), the index and its result links live under it too.
      const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";
      const index = process.env.NODE_ENV === "development" ? "/pagefind-dev/pagefind.js" : `${base}/pagefind/pagefind.js`;
      const load = new Function("url", "return import(url)");
      const pf = (await load(index)) as Pagefind;
      await pf.options({ baseUrl: `${base}/` });
      pf.init();
      return pf;
    } catch {
      return null;
    }
  })();
  return pagefindPromise;
}

type Row = { chapter: string; tab: string; url: string; excerpt: string };

export function SearchButton() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[] | null>(null);
  const [missing, setMissing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((v) => !v);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const run = (q: string) => {
    setQuery(q);
    clearTimeout(timer.current);
    if (!q.trim()) {
      setRows(null);
      return;
    }
    timer.current = setTimeout(async () => {
      const pf = await loadPagefind();
      if (!pf) {
        setMissing(true);
        setRows([]);
        return;
      }
      const found = await pf.search(q);
      const out: Row[] = [];
      for (const r of found.results.slice(0, 5)) {
        const data = await r.data();
        const chapter = data.meta.title ?? data.url;
        const subs = data.sub_results.length ? data.sub_results.slice(0, 3) : [];
        if (subs.length === 0) out.push({ chapter, tab: "", url: data.url, excerpt: data.excerpt });
        for (const sub of subs) {
          out.push({ chapter, tab: sub.title, url: sub.url, excerpt: sub.excerpt });
        }
      }
      setRows(out.slice(0, 8));
    }, 120);
  };

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        aria-label="Search the documentation"
        className="flex cursor-pointer items-center gap-2 rounded-lg border border-line bg-face px-[13px] py-[7px] text-[13px] text-soft max-lg:h-10 max-lg:w-10 max-lg:justify-center max-lg:p-0"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true" className="max-lg:h-[17px] max-lg:w-[17px]">
          <circle cx="11" cy="11" r="7" stroke="currentColor" strokeWidth="2" />
          <path d="m20 20-3.5-3.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <span className="max-lg:hidden">Search</span>
        <span className="rounded border border-line px-[5px] py-px font-mono text-[10.5px] max-lg:hidden">Ctrl K</span>
      </button>

      {open && (
        <div className="absolute right-0 top-[46px] z-40 w-[460px] max-w-[92vw] rounded-xl border border-line bg-panel p-3 shadow-[0_10px_28px_rgba(0,0,0,0.14)] max-sm:fixed max-sm:inset-x-3 max-sm:top-[64px] max-sm:w-auto max-sm:max-w-none">
          <div className="mb-2.5 flex items-center gap-2 rounded-lg border-[1.5px] border-line px-3 py-2 focus-within:border-[var(--ink)]">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <circle cx="11" cy="11" r="7" stroke="var(--soft)" strokeWidth="2" />
              <path d="m20 20-3.5-3.5" stroke="var(--soft)" strokeWidth="2" strokeLinecap="round" />
            </svg>
            <input
              ref={inputRef}
              value={query}
              onChange={(e) => run(e.target.value)}
              placeholder="Search every chapter…"
              aria-label="Search the documentation"
              className="w-full border-0 bg-transparent text-[13.5px] text-ink outline-none"
            />
          </div>

          {rows === null && (
            <p className="m-0 px-1 text-[11.5px] text-soft">
              Results open the page, at the part that matches. Search runs in your browser, with no server.
            </p>
          )}
          {rows !== null && missing && (
            <p className="m-0 px-1 text-[12px] text-soft">
              The search index is missing. In development, start the site with{" "}
              <code className="font-mono">npm run dev</code>, which builds it first.
            </p>
          )}
          {rows !== null && !missing && rows.length === 0 && (
            <p className="m-0 px-1 text-[12.5px] text-soft">Nothing matches. Try another word.</p>
          )}
          {rows !== null && rows.length > 0 && (
            <div className="flex max-h-[min(320px,60vh)] flex-col gap-1.5 overflow-y-auto">
              {rows.map((row, i) => (
                <a
                  key={i}
                  href={row.url}
                  onClick={() => setOpen(false)}
                  className="block rounded-lg px-[11px] py-[9px] no-underline hover:bg-active"
                >
                  <p className="m-0 mb-0.5 flex items-center gap-1.5">
                    <span className="text-[12.5px] font-extrabold text-ink">{row.chapter}</span>
                    {row.tab && (
                      <span className="rounded bg-chip px-1.5 py-px text-[10.5px] font-bold text-muted">
                        {row.tab}
                      </span>
                    )}
                  </p>
                  <p
                    className="m-0 text-[12.5px] leading-normal text-muted [&_mark]:bg-transparent [&_mark]:font-bold [&_mark]:text-accent-text"
                    dangerouslySetInnerHTML={{ __html: row.excerpt }}
                  />
                </a>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
