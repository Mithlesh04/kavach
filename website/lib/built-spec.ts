// The spec describes some things before their code lands. It marks them, and
// the site leaves them out, so every page shows only what works today:
//
// - A file or section whose first line under its heading is exactly
//   `*Not built yet.*` is left out, up to the next heading of the same or a
//   higher level. A file keeps its title line, so it is still found.
// - A table row whose last cell ends with "(not built yet)" is left out.
//
// The marker is removed in the spec when the code lands.

export const NOT_BUILT = "*Not built yet.*";

const ROW_NOT_BUILT = /\(not built yet\)\s*\|?\s*$/;

/** `text`, a spec file, without what the spec marks as not built yet. */
export function builtOnly(text: string): string {
  const lines = text.split("\n");
  const out: string[] = [];
  let dropping = 0; // the level of the heading being left out; 0 when none
  let fenced = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trimStart().startsWith("```")) fenced = !fenced;
    const heading = fenced ? null : line.match(/^(#{1,6}) /);
    if (heading && dropping && heading[1].length <= dropping) dropping = 0;
    if (dropping) continue;
    if (heading) {
      let next = i + 1;
      while (next < lines.length && !lines[next].trim()) next++;
      if (lines[next]?.trim() === NOT_BUILT) {
        dropping = heading[1].length;
        if (dropping === 1) out.push(line);
        continue;
      }
    }
    if (!fenced && line.trimStart().startsWith("|") && ROW_NOT_BUILT.test(line)) continue;
    out.push(line);
  }
  return out.join("\n");
}
