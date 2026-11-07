import fs from "node:fs";
import path from "node:path";

// The fields of a std.ai record (`Model`, `AgentSettings`, `Audit`), read
// from stdlib/ai/ai.kav at build time: each field's name, type, and doc
// comment, as the library declares them. A field added there shows up
// here on the next build.

export type ModelField = { name: string; type: string; optional: boolean; doc: string };

const FILE = "stdlib/ai/ai.kav";

/** The fields of `pub type <name> { … }` in std.ai. */
export function typeFields(name: string): ModelField[] {
  const text = fs.readFileSync(path.join(process.cwd(), "..", FILE), "utf8").replace(/\r\n/g, "\n");
  const start = text.indexOf(`pub type ${name} {`);
  if (start < 0) throw new Error(`${FILE}: no pub type ${name}`);
  const body = text.slice(start, text.indexOf("\n}", start)).split("\n").slice(1);
  const fields: ModelField[] = [];
  let doc: string[] = [];
  for (const line of body.map((l) => l.trim())) {
    if (line.startsWith("///")) {
      doc.push(line.slice(3).trim());
      continue;
    }
    const m = line.match(/^pub (\w+)(\?)?: (.+)$/);
    if (m) fields.push({ name: m[1], type: m[3], optional: !!m[2], doc: doc.join(" ") });
    doc = [];
  }
  if (!fields.length) throw new Error(`${FILE}: ${name} has no fields`);
  return fields;
}

export const modelFields = () => typeFields("Model");

/** The choices of `pub type <name> = A | B | …` in std.ai. */
export function typeChoices(name: string): string[] {
  const text = fs.readFileSync(path.join(process.cwd(), "..", FILE), "utf8");
  const m = text.match(new RegExp(`pub type ${name} = ([^\\n]+)`));
  if (!m) throw new Error(`${FILE}: no pub type ${name} = …`);
  return m[1].split("|").map((c) => c.trim());
}
