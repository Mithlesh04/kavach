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
    if (e === "\n") {
      // A `\` at the end of a line goes on with the next line's first non-space.
      while (/\s/.test(text[i + 1] ?? "")) i++;
      continue;
    }
    out += e === "n" ? "\n" : e === "t" ? "\t" : e;
  }
  throw new Error(`${where}: the string never ends`);
}

/** Where `fn name` starts in a Rust test file, and the file's text. */
function fnAt(rel: string, name: string): { text: string; at: number } {
  const text = read(rel);
  const at = text.indexOf(`fn ${name}(`);
  if (at < 0) throw new Error(`${rel}: no fn ${name}`);
  return { text, at };
}

/** The string a test function sets with `let name = "…"`. */
export function rustLet(rel: string, fn: string, name: string): string {
  return rustStringAfter(rel, fn, `let ${name} =`);
}

/** The first string literal at or after `marker` in a test function, such as `out, "`. */
export function rustStringAfter(rel: string, fn: string, marker: string): string {
  const { text, at } = fnAt(rel, fn);
  return literal(text, text.indexOf(marker, at), `${rel} ${fn} ${marker}`);
}

/** The first raw string, `r#"…"#`, after `marker` in a test function. */
export function rustRaw(rel: string, fn: string, marker: string): string {
  const { text, at } = fnAt(rel, fn);
  const from = text.indexOf(marker, at);
  const start = from < 0 ? -1 : text.indexOf('r#"', from);
  const end = start < 0 ? -1 : text.indexOf('"#', start + 3);
  if (end < 0) throw new Error(`${rel} ${fn}: no r#"…"# after ${marker}`);
  return text.slice(start + 3, end);
}

/** Every string literal of a test function from `marker` to the `];` that ends its list, in order. */
export function rustStrings(rel: string, fn: string, marker: string): string[] {
  const { text, at } = fnAt(rel, fn);
  let from = text.indexOf(marker, at);
  const end = text.indexOf("];", from);
  if (from < 0 || end < 0) throw new Error(`${rel} ${fn}: no ${marker} … ];`);
  const out: string[] = [];
  for (;;) {
    const start = text.indexOf('"', from);
    if (start < 0 || start > end) return out;
    const s = literal(text, start, `${rel} ${fn}`);
    out.push(s);
    from = closingQuote(text, start) + 1;
  }
}

/** The index of the `"` that closes the literal opening at `start`. */
function closingQuote(text: string, start: number): number {
  for (let i = start + 1; i < text.length; i++) {
    if (text[i] === "\\") i++;
    else if (text[i] === '"') return i;
  }
  throw new Error("a string never ends");
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
