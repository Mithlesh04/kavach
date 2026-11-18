import type { ReactNode } from "react";

/**
 * A word of code inside prose: `let`, `match`, `0..=10`. Sized relative
 * to the sentence around it, so one style fits every paragraph.
 */
export function C({ children }: { children: ReactNode }) {
  return <code className="rounded bg-chip px-1.5 py-px font-mono text-[0.9em]">{children}</code>;
}

/**
 * Text with Markdown code spans, as the spec and READMEs write them: a run
 * of backticks opens code and the next run of the same length closes it,
 * so `` `Hello, {name}` `` holds a template's backticks. One space just
 * inside both ends is dropped. `plain` shapes the text between the spans.
 */
export function withCode(text: string, plain: (part: string) => ReactNode = (p) => p): ReactNode[] {
  const out: ReactNode[] = [];
  let rest = text;
  while (rest !== "") {
    const open = rest.match(/`+/);
    if (!open || open.index === undefined) break;
    const run = open[0];
    const after = rest.slice(open.index + run.length);
    const close = after.match(new RegExp(`(?<!\`)${run}(?!\`)`));
    if (!close || close.index === undefined) break;
    if (open.index > 0) out.push(plain(rest.slice(0, open.index)));
    let code = after.slice(0, close.index);
    if (code.length > 2 && code.startsWith(" ") && code.endsWith(" ")) code = code.slice(1, -1);
    out.push(<C key={out.length}>{code}</C>);
    rest = after.slice(close.index + run.length);
  }
  if (rest !== "") out.push(plain(rest));
  return out;
}
