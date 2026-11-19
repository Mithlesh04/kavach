// The spec describes some things before their code lands. It marks them, and
// the site leaves them out, so every page shows only what works today:
//
// - A file or section whose first line under its heading is exactly
//   `*Not built yet.*` is left out, up to the next heading of the same or a
//   higher level. A file keeps its title line, so it is still found.
// - A paragraph that starts with `*Not built yet:*` is left out, with the
//   code blocks and tables right after it.
// - A list item that starts with `*Not built yet:*` is left out, with the
//   lines indented under it.
// - A table row whose last cell ends with "(not built yet)" is left out.
//
// The marker is removed in the spec when the code lands.
//
// Some text mixes what works with what doesn't, in one table row or one
// sentence, where no marker fits. The site reads those files as they were
// before the design, in PINNED, until its code lands, with the changes of
// later commits that built something laid on top.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const REPO = path.join(process.cwd(), "..");

/** The commit before decisions 0135–0137 (tables, one Tensor type, numbers in types) were written into the spec. */
const BEFORE_DESIGNS = "81e236e3";

/** The commits that wrote those designs into the spec, and marked them. */
const DESIGNS = ["f35e8eb6", "9357ff02"];

/** Slicing, decision 0134: built, so its changes to a pinned file are shown. */
const SLICING = "d570b0d2";

/**
 * Spec files read as they were at a commit (`at`), with the changes of the
 * `built` commits since laid on top; `DESIGNS` may change them too. Any
 * other commit that changes one fails the build, so built text there is
 * never missed: look at what it changed, then add it to `built`, move the
 * pin, or take the file out.
 */
const PINNED: Record<string, { at: string; built: string[] }> = Object.fromEntries(
  (
    [
      ["02-types/01-built-in-types.md", []], // Int8 and Decimal rows name Int<bits: n> and Decimal<digits, places>
      ["02-types/17-matrices.md", []], // says Matrix is gone; it still works until Tensor lands
      ["14-standard-library/05-math.md", []], // math.mean still works
      ["14-standard-library/09-text-and-collections.md", [SLICING]], // collections.unique and counts still work
    ] as [string, string[]][]
  ).map(([f, built]) => [`docs/spec/${f}`, { at: BEFORE_DESIGNS, built }]),
);

const git = (args: string[]) => execFileSync("git", args, { cwd: REPO, encoding: "utf8" });
const same = (a: string, b: string) => a.startsWith(b) || b.startsWith(a);

for (const [rel, { at, built }] of Object.entries(PINNED)) {
  const changed = git(["log", "--format=%h", `${at}..HEAD`, "--", rel]).split("\n").filter(Boolean);
  const other = changed.filter((c) => ![...DESIGNS, ...built].some((s) => same(c, s)));
  if (other.length) throw new Error(`${rel} is pinned at ${at}, and ${other.join(", ")} changed it since: see what it built, then move the pin (lib/built-spec.ts)`);
}

const show = (commit: string, rel: string) => git(["show", `${commit}:${rel}`]).replace(/\r\n/g, "\n");

/** `text` with the change `commit` made to `rel`, by a three-way merge; a change that doesn't fit fails the build. */
function withChange(text: string, rel: string, commit: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kavach-spec-"));
  const [ours, base, theirs] = ["ours", "base", "theirs"].map((n) => path.join(dir, n));
  try {
    fs.writeFileSync(ours, text);
    fs.writeFileSync(base, show(`${commit}^`, rel));
    fs.writeFileSync(theirs, show(commit, rel));
    return execFileSync("git", ["merge-file", "-p", ours, base, theirs], { encoding: "utf8" });
  } catch {
    throw new Error(`${rel}: ${commit}'s change doesn't fit the text pinned (lib/built-spec.ts)`);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

/** A spec file (`docs/spec/…`) as the site shows it: pinned, or without what is marked as not built yet. */
export function specText(rel: string): string {
  const pin = PINNED[rel];
  if (!pin) return builtOnly(fs.readFileSync(path.join(REPO, rel), "utf8").replace(/\r\n/g, "\n"));
  if (!pinnedText.has(rel)) pinnedText.set(rel, pin.built.reduce((text, c) => withChange(text, rel, c), show(pin.at, rel)));
  return pinnedText.get(rel)!;
}

const pinnedText = new Map<string, string>();

export const NOT_BUILT = "*Not built yet.*";

const PARAGRAPH = "*Not built yet:*";
const ITEM = /^(\s*)[-*] \*Not built yet:\*/;
const ROW = /\(not built yet\)\s*\|?\s*$/;

const indentOf = (line: string) => line.length - line.trimStart().length;
const isFence = (line: string) => line.trimStart().startsWith("```");

/** `text`, a spec file, without what the spec marks as not built yet. */
export function builtOnly(text: string): string {
  const lines = text.split("\n");
  const out: string[] = [];
  let i = 0;
  /** The index after the fenced block that starts at `from`. */
  const pastFence = (from: number) => {
    let j = from + 1;
    while (j < lines.length && !isFence(lines[j])) j++;
    return j + 1;
  };
  /** The next line that isn't blank, from `from`. */
  const nextText = (from: number) => {
    let j = from;
    while (j < lines.length && !lines[j].trim()) j++;
    return j;
  };
  /** Past the marked paragraph at `from`, and the code blocks and tables right after it. */
  const droppedParagraph = (from: number): number => {
    let j = from;
    while (j < lines.length && lines[j].trim()) j++;
    for (;;) {
      const next = nextText(j);
      if (next >= lines.length) return next;
      if (isFence(lines[next])) j = pastFence(next);
      else if (lines[next].trimStart().startsWith("|")) {
        j = next;
        while (j < lines.length && lines[j].trimStart().startsWith("|")) j++;
      } else return j;
    }
  };
  while (i < lines.length) {
    const line = lines[i];
    if (isFence(line)) {
      const end = pastFence(i);
      out.push(...lines.slice(i, end));
      i = end;
      continue;
    }
    const heading = line.match(/^(#{1,6}) /);
    if (heading && lines[nextText(i + 1)]?.trim() === NOT_BUILT) {
      const level = heading[1].length;
      if (level === 1) out.push(line);
      i++;
      while (i < lines.length) {
        if (isFence(lines[i])) {
          i = pastFence(i);
          continue;
        }
        const h = lines[i].match(/^(#{1,6}) /);
        if (h && h[1].length <= level) break;
        i++;
      }
      continue;
    }
    if (line.startsWith(PARAGRAPH)) {
      i = droppedParagraph(i);
      continue;
    }
    const item = line.match(ITEM);
    if (item) {
      const indent = item[1].length;
      i++;
      while (i < lines.length) {
        const next = nextText(i);
        if (next >= lines.length || indentOf(lines[next]) <= indent) break;
        i = isFence(lines[next]) ? pastFence(next) : next + 1;
      }
      continue;
    }
    if (line.trimStart().startsWith("|") && ROW.test(line)) {
      i++;
      continue;
    }
    out.push(line);
    i++;
  }
  return out.join("\n");
}
