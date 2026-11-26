"use client";

// What a reader's browser keeps about their learning: the path they follow
// and the pages they have read. Only in this browser, never sent anywhere,
// no account. Progress belongs to pages, not paths, so switching paths keeps
// every tick. Storage can be blocked or empty (a private window): every
// read and write is guarded, and the site works the same without it.

export type PathPage = { href: string; title: string };
export type PathData = { id: string; title: string; pages: PathPage[] };

const PATH_KEY = "kavach-path";
const READ_KEY = "kavach-read";
/** Pages read and path changes are announced, so every part of the page follows at once. */
export const CHANGED = "kavach-learning-changed";

function get(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function set(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // Storage is blocked: the choice lasts for this page only.
  }
  window.dispatchEvent(new Event(CHANGED));
}

export const chosenPath = () => get(PATH_KEY);
export const choosePath = (id: string | null) => set(PATH_KEY, id);

export function readPages(): Set<string> {
  try {
    return new Set(JSON.parse(get(READ_KEY) ?? "[]") as string[]);
  } catch {
    return new Set();
  }
}

export function markRead(href: string) {
  const read = readPages();
  if (read.has(href)) return;
  read.add(href);
  set(READ_KEY, JSON.stringify([...read]));
}

let loading: Promise<PathData[]> | undefined;

/** Every learning path, from /paths.json, fetched once a page load. */
export function loadPaths(): Promise<PathData[]> {
  const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";
  loading ??= fetch(`${base}/paths.json`)
    .then((r) => r.json() as Promise<{ paths: PathData[] }>)
    .then((j) => j.paths)
    .catch(() => []);
  return loading;
}
