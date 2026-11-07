// Where the kavach program is, for the scripts that run it at check time:
// KAVACH_BIN when set, else the repository's own release or debug build.
import fs from "node:fs";
import path from "node:path";

/** The kavach program's path, or null when there is none. */
export function findKavach(root) {
  if (process.env.KAVACH_BIN && fs.existsSync(process.env.KAVACH_BIN)) return process.env.KAVACH_BIN;
  const exe = process.platform === "win32" ? ".exe" : "";
  for (const p of [
    path.join(root, "..", "target", "release", `kavach${exe}`),
    path.join(root, "..", "target", "debug", `kavach${exe}`),
  ]) {
    if (fs.existsSync(p)) return p;
  }
  return null;
}
