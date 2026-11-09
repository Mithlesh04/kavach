import fs from "node:fs";
import path from "node:path";

// Programs and outputs that only a Rust test can run, such as an `ask`
// against a fake model server, read from the test itself at build time, so
// the page shows exactly what the test checks and is never typed.

const read = (rel: string) => fs.readFileSync(path.join(process.cwd(), "..", rel), "utf8").replace(/\r\n/g, "\n");

/** The Rust string literal that starts at the first `"` after `from`. */
function literal(text: string, from: number, where: string): string {
  const start = text.indexOf('"', from);
  if (from < 0 || start < 0) throw new Error(`${where}: not found`);
  let out = "";
  for (let i = start + 1; i < text.length; i++) {
    const c = text[i];
    if (c === '"') return out;
    if (c !== "\\") {
      out += c;
      continue;
    }
    const e = text[++i];
    out += e === "n" ? "\n" : e === "t" ? "\t" : e;
  }
  throw new Error(`${where}: the string never ends`);
}

/** A `const NAME: &str = "…";` of a Rust test file. */
export function rustConst(rel: string, name: string): string {
  const text = read(rel);
  return literal(text, text.indexOf(`const ${name}: &str =`), `${rel} ${name}`);
}

/** The output a test function checks: the first string after `out,` in `fn name`. */
export function rustExpected(rel: string, name: string): string {
  const text = read(rel);
  const fn = text.indexOf(`fn ${name}(`);
  return literal(text, fn < 0 ? -1 : text.indexOf("out,", fn), `${rel} ${name}`);
}
