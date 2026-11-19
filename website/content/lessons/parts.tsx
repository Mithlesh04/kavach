import { isValidElement, type ReactNode } from "react";
import fs from "node:fs";
import path from "node:path";
import { CodeBlock } from "@/components/code-block";
import { SrcRef } from "@/components/src-ref";
import type { HoverNote } from "@/lib/chapter";
import { loadProjectFiles, loadSample } from "@/lib/samples";
import { specBlock, specBlockWith } from "@/lib/spec-tables";

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

/**
 * A code block of the spec, shown and not run, as the spec writes it: the
 * one in section `number` of `file` (under docs/spec/) that holds `text`,
 * or the section's first block. Its tab shows `name`, the file the code
 * would be in, when the page gives one (`main.kav`, `kavach.toml`); the link
 * under it names the spec file and the line the code starts on.
 */
export function SpecCode({
  file,
  number,
  text,
  name,
  lang,
}: {
  file: string;
  number: string;
  text?: string;
  name?: string;
  lang?: string;
}) {
  const code = text === undefined ? specBlock(file, number) : specBlockWith(file, number, text);
  const rel = `docs/spec/${file}`;
  const raw = fs.readFileSync(path.join(process.cwd(), "..", rel), "utf8").split(/\r?\n/);
  // The block's first line: the line after a fence where every line of the code follows.
  const lines = code.split(/\r?\n/).map((l) => l.trim());
  const at = raw.findIndex(
    (_, i) => raw[i - 1]?.trim().startsWith("```") && lines.every((l, k) => raw[i + k]?.trim() === l),
  );
  return (
    <div className="flex flex-col gap-2">
      <CodeBlock code={code} file={name} lang={lang} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={rel} line={at >= 0 ? at + 1 : undefined} label={`from spec ${number}`} />
      </div>
    </div>
  );
}
