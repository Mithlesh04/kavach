"use client";

import { useEffect, useState } from "react";
import versions from "@/data/versions.json";

// The docs version this build is: `latest` for the ordinary build, the
// entry's id for one scripts/build-versions.mjs builds from a git tag.
const current = process.env.NEXT_PUBLIC_DOCS_VERSION || versions.latest;
const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

/** Where version `id` is served: latest at the root, the others under /<id>/. */
const rootOf = (id: string) => (id === versions.latest ? "" : `/${id}`);

/**
 * Opens the page the reader is on in version `id`, or that version's home
 * when it has no such page: a page may be newer or older than the version.
 */
async function go(id: string) {
  const here = window.location.pathname.slice(base.length) || "/";
  // Another version is another build with its own base path, so this is a
  // full page load to an absolute address, not a move inside this one.
  const root = `${window.location.origin}${rootOf(id)}`;
  // The same page, else Kavach Home, which every version has, else its root.
  let target = `${root}/`;
  for (const page of [`${root}${here}`, `${root}/docs/start/kavach-home/`]) {
    try {
      if ((await fetch(page, { method: "HEAD" })).ok) {
        target = page;
        break;
      }
    } catch {
      // Offline or blocked: try the next place.
    }
  }
  window.location.assign(target);
}

/**
 * An older version's pages say which version they describe, in a bar at
 * the top. It is added once the page has loaded, outside React's tree,
 * as an older build's own layout has no place for it.
 */
function markOlder() {
  if (document.getElementById("older-version")) return;
  const tag = versions.versions.find((v) => v.id === current)?.tag ?? current;
  const bar = document.createElement("div");
  bar.id = "older-version";
  bar.className = "border-b border-line bg-accent-soft px-4 py-2 text-center text-sm text-ink";
  bar.append(`These are the docs for Kavach ${current}, as released at tag ${tag}. `);
  const latest = document.createElement("a");
  latest.href = `${window.location.origin}/`;
  latest.className = "font-bold text-accent-text";
  latest.textContent = "Read the latest docs";
  bar.append(latest);
  document.body.prepend(bar);
}

export function VersionPicker() {
  const [open, setOpen] = useState(false);
  const old = current !== versions.latest;
  useEffect(() => {
    if (old) markOlder();
  }, [old]);
  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        aria-haspopup="listbox"
        aria-expanded={open}
        className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-face px-[11px] py-1.5 font-mono text-[12.5px] font-bold text-ink max-lg:h-10"
      >
        {current}{" "}
        <span className={`font-medium max-sm:hidden ${old ? "text-accent" : "text-soft"}`}>{old ? "older" : "latest"}</span>
        <svg width="10" height="6" viewBox="0 0 10 6" aria-hidden="true">
          <path d="m1 1 4 4 4-4" stroke="currentColor" strokeWidth="1.6" fill="none" />
        </svg>
      </button>
      {open && (
        <div
          role="listbox"
          aria-label="Documentation versions"
          className="absolute left-0 top-[46px] z-40 w-80 rounded-[10px] border border-line bg-panel p-2 shadow-[0_10px_28px_rgba(0,0,0,0.14)] max-sm:fixed max-sm:inset-x-3 max-sm:top-[64px] max-sm:w-auto"
        >
          {versions.versions.map((v) => (
            <button
              key={v.id}
              role="option"
              aria-selected={v.id === current}
              onClick={() => (v.id === current ? setOpen(false) : go(v.id))}
              className={`block w-full cursor-pointer rounded-lg px-[11px] py-[9px] text-left hover:bg-active ${v.id === current ? "bg-active" : ""}`}
            >
              <span className="block text-[13.5px] font-extrabold text-ink">
                {v.id}
                {v.id === versions.latest && <span className="font-semibold text-muted"> · latest</span>}
              </span>
              <span className="mt-0.5 block text-xs text-soft">
                {v.tag === "main" ? "the newest source, from main" : `as released, from tag ${v.tag}`}
              </span>
            </button>
          ))}
          <div className="mx-1 mb-0.5 mt-1.5 border-t border-line-2 px-[7px] pt-2">
            <p className="m-0 text-[11.5px] leading-normal text-soft">
              Each version&apos;s pages and file links show the code of that version. A page that a version
              doesn&apos;t have opens that version&apos;s home.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
