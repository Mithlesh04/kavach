"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { CHANGED, choosePath, chosenPath } from "./path-store";

/** Follows a learning path: remembers it in this browser and opens its first page. */
export function PathFollow({ id, first }: { id: string; first: string }) {
  const router = useRouter();
  const [chosen, setChosen] = useState<string | null>(null);
  useEffect(() => {
    const load = () => setChosen(chosenPath());
    load();
    window.addEventListener(CHANGED, load);
    return () => window.removeEventListener(CHANGED, load);
  }, []);
  if (chosen === id) {
    return (
      <span className="flex flex-wrap items-center gap-3 text-sm">
        <span className="font-bold text-ink">You follow this path.</span>
        <button type="button" onClick={() => choosePath(null)} className="cursor-pointer border-0 bg-transparent p-0 text-soft underline">
          Leave it
        </button>
      </span>
    );
  }
  return (
    <button
      type="button"
      onClick={() => {
        choosePath(id);
        router.push(first);
      }}
      className="cursor-pointer rounded-lg border-0 bg-accent px-4 py-2 text-sm font-bold text-bg"
    >
      Follow this path
    </button>
  );
}
