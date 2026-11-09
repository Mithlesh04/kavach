// Every limit and size the course states, in one place (Mithlesh, 2026-11-04):
// each topic page shows its own rows (content/lessons/limits.tsx), and the
// Limits and Sizes page shows them all, each linking back to the page that
// explains it. So a number is written once and can't disagree with itself.
// Each row names where it comes from; the recursion limits have their own,
// more detailed table on the Recursion page, which the reference links to.

export type Limit = { what: string; value: string; source: string };
export type LimitArea = { id: string; title: string; page: string; rows: Limit[] };

const spec = (file: string) => `docs/spec/${file}`;

export const limitAreas: LimitArea[] = [
  {
    id: "numbers",
    title: "Numbers",
    page: "basics/data-types",
    rows: [
      { what: "Int", value: "-9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 (64 bits); going past it panics", source: spec("02-types/01-built-in-types.md") },
      { what: "Int8, Int16, Int32", value: "-128 to 127; -32,768 to 32,767; -2,147,483,648 to 2,147,483,647", source: spec("02-types/01-built-in-types.md") },
      { what: "UInt8, UInt16, UInt32, UInt64", value: "0 to 255; 0 to 65,535; 0 to 4,294,967,295; 0 to 18,446,744,073,709,551,615", source: spec("02-types/01-built-in-types.md") },
      { what: "Float", value: "64 bits: about 15 to 17 significant digits, up to about 1.8e308", source: spec("02-types/01-built-in-types.md") },
      { what: "Float32", value: "32 bits: about 7 significant digits, up to about 3.4e38", source: spec("02-types/01-built-in-types.md") },
      { what: "Decimal", value: "no limit: up to about 38 digits in a fast 128-bit form, more in a bigger one; a division that doesn't end keeps 18 digits after the point", source: spec("02-types/01-built-in-types.md") },
      { what: "BigInt", value: "no limit: as many digits as the number needs", source: spec("02-types/01-built-in-types.md") },
    ],
  },
  {
    id: "text",
    title: "Text",
    page: "basics/strings",
    rows: [
      { what: "String", value: "UTF-8 text, as long as memory allows; it can't be changed in place", source: spec("02-types/01-built-in-types.md") },
      { what: "s.len()", value: "counts characters, not bytes: \"नमस्ते\".len() is 6", source: spec("14-standard-library/02-built-in-methods.md") },
      { what: "Char", value: "one Unicode character: code points 0 to 0x10FFFF, surrogates excluded", source: spec("02-types/01-built-in-types.md") },
    ],
  },
  {
    id: "lists",
    title: "Lists",
    page: "collections/lists",
    rows: [
      { what: "[UInt8], [Int8], [Bool]", value: "1 byte per item", source: "docs/decisions/0030-byte-packed-lists.md" },
      { what: "[Int16], [UInt16]; [Int32], [UInt32], [Float32]", value: "2 bytes; 4 bytes per item", source: "docs/decisions/0045-packed-lists-of-small-numbers.md" },
      { what: "Other items", value: "8 bytes per item: a 64-bit number, or a reference to a bigger value. Packing is in native code; the interpreter keeps its own values", source: "docs/decisions/0045-packed-lists-of-small-numbers.md" },
    ],
  },
  {
    id: "tasks",
    title: "Tasks and channels",
    page: "concurrency/channels",
    rows: [
      { what: "Channel capacity", value: "1 or more; put waits while the channel is full", source: spec("09-concurrency.md") },
      { what: "A task's stack", value: "512 MB of address space each, used only as calls need it", source: "runtime/native/src/lib.rs" },
    ],
  },
  {
    id: "network",
    title: "Network",
    page: "networks/sockets",
    rows: [
      { what: "Any network wait (not accept or receive)", value: "gives up after 30 seconds, with TimedOut; .timeout(d) changes it", source: spec("14-standard-library/10-net.md") },
      { what: "conn.read()", value: "at most 64 KB at a time", source: spec("14-standard-library/10-net.md") },
      { what: "A UDP datagram", value: "at most 64 KB is read", source: spec("14-standard-library/10-net.md") },
      { what: "An HTTP server's idle connection", value: "closed after 5 seconds without a request", source: spec("14-standard-library/11-http.md") },
    ],
  },
  {
    id: "json",
    title: "JSON",
    page: "standard-library/json",
    rows: [
      { what: "Nesting", value: "128 levels deep; max_depth raises it, up to 10,000", source: spec("02-types/12-json.md") },
      { what: "Text", value: "64 MB; max_bytes raises it", source: spec("02-types/12-json.md") },
      { what: "Fields in one object", value: "100,000; max_fields raises it", source: spec("02-types/12-json.md") },
      { what: "A number's exponent", value: "from 10^-10,000 to 10^10,000, for decimal() and bigint()", source: spec("02-types/12-json.md") },
    ],
  },
  {
    id: "random",
    title: "Random numbers",
    page: "standard-library/random-numbers",
    rows: [{ what: "random.bytes(n)", value: "n from 0 to 65,536 per call", source: spec("14-standard-library/15-random.md") }],
  },
  {
    id: "logging",
    title: "Logging",
    page: "standard-library/logging",
    rows: [
      { what: "A log file", value: "a new file after 10 MB (max_bytes); 5 older ones kept (keep)", source: spec("14-standard-library/12-log.md") },
      { what: "Events waiting for an OpenTelemetry collector", value: "up to 10,000, sent at least once a second; at exit, up to 2 seconds to send the rest", source: spec("14-standard-library/12-log.md") },
    ],
  },
  {
    id: "ai",
    title: "AI calls",
    page: "agents/ask",
    rows: [
      { what: "The model", value: "claude-haiku-5-5, unless the agent (model) or the call (model:) chooses one", source: spec("08-capabilities-and-agents/04-typed-ai-calls.md") },
      { what: "An answer's length", value: "4,096 tokens, reasoning included; a Model's max_tokens changes it", source: spec("08-capabilities-and-agents/04-typed-ai-calls.md") },
      { what: "An answer that doesn't fit the type", value: "asked again 2 times, then InvalidOutput", source: spec("08-capabilities-and-agents/04-typed-ai-calls.md") },
    ],
  },
  {
    id: "gpu",
    title: "GPU",
    page: "speed/gpu/gpu-fn",
    rows: [
      { what: "Values in a gpu fn", value: "Float32, Int32, UInt32 and Bool only", source: spec("15-gpu.md") },
      { what: "A record or tuple item", value: "1 to 8 fields of those types", source: spec("15-gpu.md") },
    ],
  },
  {
    id: "files",
    title: "Files and functions",
    page: "tools/small-files",
    rows: [
      { what: "A .kav file", value: "500 lines, counting every line: an error by default", source: spec("12-maintainability.md") },
      { what: "A function body", value: "80 lines: a warning", source: spec("12-maintainability.md") },
    ],
  },
];
