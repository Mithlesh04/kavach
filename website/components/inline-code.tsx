import type { ReactNode } from "react";

/**
 * A word of code inside prose: `let`, `match`, `0..=10`. Sized relative
 * to the sentence around it, so one style fits every paragraph.
 */
export function C({ children }: { children: ReactNode }) {
  return <code className="rounded bg-chip px-1.5 py-px font-mono text-[0.9em]">{children}</code>;
}
