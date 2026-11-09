import { isValidElement, type ReactNode } from "react";
import { CodeBlock } from "@/components/code-block";
import type { HoverNote } from "@/lib/chapter";
import { loadProjectFiles, loadSample } from "@/lib/samples";

// Small building blocks every lesson page uses, so pages stay prose and
// samples. Samples come from content/samples/ and are run by the real
// kavach on every check (decision 0033).

/** A whole sample program with what it prints (or the compiler's refusal). */
export function Ex({ name, notes }: { name: string; notes?: HoverNote[] }) {
  const s = loadSample(name);
  const outcome = name.endsWith(".fails")
    ? "refused"
    : name.endsWith(".panics")
      ? "panic"
      : name.endsWith(".warns")
        ? "warned"
        : "output";
  // A `.native` sample runs compiled: the window shows the command that printed its output.
  const run = name.endsWith(".native") ? `kavach run --native ${s.file}` : undefined;
  return <CodeBlock code={s.code} file={s.file} output={s.output} outcome={outcome} notes={notes} run={run} />;
}

/**
 * A project of several files in one window, a tab for each: the one the page
 * talks about first (`first`), then src/main.kav, then the rest. `terminal` is
 * false for a story across runs, shown in its own `Terminal` below.
 */
export function Files({
  name,
  first,
  outcome,
  terminal = true,
}: {
  name: string;
  first?: string[];
  outcome?: "output" | "refused" | "panic" | "warned";
  terminal?: boolean;
}) {
  const { files, output } = loadProjectFiles(name, first);
  return <CodeBlock code={files[0].code} file={files[0].path} also={files.slice(1)} output={terminal ? output : undefined} outcome={outcome} />;
}

/** The shape of a construct, before the examples. */
export function Syntax({ code }: { code: string }) {
  return <CodeBlock code={code} file="syntax" />;
}

/** A heading's words, for its address: text inside `<C>` and other elements counts. */
function wordsOf(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(wordsOf).join("");
  if (isValidElement(node)) return wordsOf((node.props as { children?: ReactNode }).children);
  return "";
}

export function H2({ children }: { children: ReactNode }) {
  const id = wordsOf(children).toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");
  return <h2 id={id} className="m-0 mt-3 text-[22px] font-extrabold tracking-tight min-[1920px]:text-[25px]">{children}</h2>;
}

export function P({ children }: { children: ReactNode }) {
  return <p className="m-0">{children}</p>;
}

/** A short aside: a rule worth remembering. */
export function Note({ children }: { children: ReactNode }) {
  return <p className="m-0 rounded-[10px] border border-line bg-panel px-4 py-3 text-[15px] text-muted">{children}</p>;
}
