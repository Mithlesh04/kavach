// The course tree (decision 0047): parts → pages → sub-pages, one idea per
// page, in the order a newcomer needs them. The sidebar, routes, sitemap,
// llms.txt and prev/next all derive from it. A page shows up on the site
// only once its content exists (lib/site.ts), so this file may list pages
// still to write. `spec` is the spec section the page teaches.

import type { Level } from "./nav";

type Raw = [title: string, spec: string, subs?: string[]];

/** URL slug from a title: "If...Else" → "if-else", "ask<T>" → "ask". */
export function slugOf(title: string): string {
  return title
    .toLowerCase()
    .replace(/&/g, " and ")
    .replace(/<[^>]*>/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

const raw: [slug: string, title: string, level: Level, pages: Raw[]][] = [
  ["start", "Start here", "beginner", [
    ["Kavach Home", "—"],
    ["Why Kavach?", "—"],
    ["What Kavach Does for You", "—"],
    ["Classic Mistakes, Fixed", "—"],
    ["Mistakes Caught Before a Run", "—"],
    ["Who Kavach Is For", "—"],
    ["What You Can Build", "—"],
    ["What a Program Is", "—"],
    ["The Terminal", "—"],
    ["Install", "—"],
    ["Your First Program", "1.3"],
    ["Reading Errors", "13"],
  ]],
  ["basics", "Basics", "beginner", [
    ["Syntax", "1.1", ["Source files", "The main function", "Blocks are values", "Names", "Keywords"]],
    ["Output", "1.10", ["print", "How values print", "debug"]],
    ["Comments", "1.2"],
    ["Variables", "1.4", ["const", "let", "Changing a value", "Destructuring"]],
    ["User Input", "14.13", ["Ask for text", "Ask for a number", "Ask for a password"]],
    ["Data Types", "2.1", ["Int", "Sized integers", "Float and Float32", "Decimal", "BigInt", "Bool", "Char", "Converting types"]],
    ["Operators", "1.8", ["Arithmetic", "Assignment", "Comparison", "Logical", "Precedence"]],
    ["Strings", "2.1", ["Making strings", "Interpolation", "Joining", "Length and characters", "Searching", "Splitting", "Strings to numbers"]],
    ["Math", "14.5", ["Math functions", "Rounding", "Exact numbers"]],
    ["Booleans", "2.1"],
    ["If...Else", "4.1", ["if", "else if", "if as a value"]],
    ["If Let", "4.2"],
    ["Match", "4.3", ["Matching values", "Patterns", "Guards"]],
    ["For Loop", "4.4", ["Over a list", "Over a range", "Over a map", "With an index", "Repeat n times"]],
    ["While Loop", "4.5", ["while", "Endless loops"]],
    ["Break/Continue", "4.6"],
  ]],
  ["functions", "Functions", "beginner", [
    ["Functions", "3.1", ["Declaring", "Parameters", "Returning a value"]],
    ["Named Arguments", "3.2", ["Named arguments", "Default values"]],
    ["Using Results", "3.7"],
    ["Lambdas", "3.3", ["Writing a lambda", "Dot shorthand"]],
    ["Functions as Values", "3.4"],
    ["Recursion", "3.8"],
    ["inout Parameters", "3.9"],
    ["Signatures", "3.5"],
    ["Deprecating a Function", "12.6"],
  ]],
  ["collections", "Collections", "beginner", [
    ["Lists", "2.2", ["Making a list", "Reading by index", "Changing items", "Adding and removing", "Sorting", "map, filter, sum"]],
    ["Maps", "2.2", ["Making a map", "Reading a value", "Adding and changing", "Looping over a map"]],
    ["Sets", "2.2", ["Making a set", "Adding and checking"]],
    ["Tuples", "2.3"],
    ["Sorting", "14.2", ["Numbers", "Text", "By a key", "Two keys"]],
  ]],
  ["types", "Your Own Types", "intermediate", [
    ["Records", "2.4", ["Declaring", "Default values", "Updating with with"]],
    ["Methods", "2.5"],
    ["Method Chains", "1.1"],
    ["Extend", "2.5"],
    ["Unions", "2.6", ["Declaring", "Matching a union"]],
    ["Optionals", "2.7", ["What an optional is", "Reading one safely"]],
    ["Anonymous Records", "2.14"],
    ["Generics", "2.9"],
    ["Interfaces", "2.8"],
    ["Interface Values", "2.15"],
    ["Classes and Inheritance", "2.8"],
    ["Writing Types", "2.10"],
  ]],
  ["errors", "Errors & Contracts", "intermediate", [
    ["Kinds of Errors", "5"],
    ["Errors", "5.1"],
    ["try", "5.2"],
    ["catch", "5.3"],
    ["Matching Errors", "5.4"],
    ["The Error Catch-all", "5.5"],
    ["Adding Context", "5.7"],
    ["Panics", "5.6", ["Panics", "When a program stops"]],
    ["Contracts", "7", ["require", "ensure", "invariant", "example", "How they are checked"]],
    ["Testing", "11", ["Tests from types", "Your own values", "test blocks", "Fakes"]],
  ]],
  ["memory", "Memory", "advanced", [
    ["Values, Not References", "6.1"],
    ["Automatic Memory", "6.1", ["How it stays fast", "Freed immediately"]],
    ["Shared State", "6.2"],
    ["Arenas", "6.3"],
    ["Unsafe", "6.4"],
  ]],
  ["standard-library", "Standard Library", "intermediate", [
    ["Files", "14.3", ["Reading", "Writing", "Folders", "Details and permissions"]],
    ["Command-line Programs", "14.6", ["Arguments", "Commands"]],
    ["The Terminal", "14.13", ["Colors and styles", "Prompts"]],
    ["Time", "14.4"],
    ["Random Numbers", "14.15"],
    ["Regex", "14.8"],
    ["Text and Collections", "14.9"],
    ["Logging", "14.12"],
    ["Agents", "14.16"],
    ["Clipboard", "14.17"],
    ["Devices", "14.18"],
    ["Cryptography", "14.19"],
    ["Base64", "14.20"],
    ["JSON", "2.12", ["Json values", "Reading", "Nested data", "Building", "Decoding into a type"]],
    ["Bytes", "2.13"],
  ]],
  ["networks", "Networks", "intermediate", [
    ["HTTP", "14.11", ["Calling an API", "Serving", "JSON over HTTP", "Sessions", "WebSockets and events"]],
    ["Sockets", "14.10", ["TCP", "UDP", "TLS", "Timeouts"]],
    ["gRPC", "14.14", ["Server streaming", "Client streaming", "Two-way and conversations", "kavach proto"]],
  ]],
  ["concurrency", "Concurrency", "advanced", [
    ["Tasks", "9.1", ["When a task fails"]],
    ["Tasks Never Leak", "9.2"],
    ["Many Things at Once", "9.3"],
    ["Channels", "9.4", ["put and take", "Many senders", "Deadlocks"]],
    ["No Data Races", "9.5"],
    ["Timeouts & Cancellation", "9.6"],
    ["Taking Turns", "9.7"],
    ["How Tasks Run", "9.8"],
  ]],
  ["agents", "Capabilities & Agents", "intermediate", [
    ["Capabilities", "8.1", ["uses", "Pure functions", "Narrower file access"]],
    ["Your Own Capabilities", "8.2", ["Declaring a capability", "Implementing one", "When a call is refused"]],
    ["Agents", "8.3", ["Declaring an agent", "model", "settings", "can", "limit", "approve", "The order of checks", "on: events"]],
    ["Running an Agent", "8.8"],
    ["ask<T>", "8.4", ["Typed answers", "When the AI fails", "Choosing the model"]],
    ["Agent Modes", "8.5", ["observe", "suggest", "enforce", "Dry runs"]],
    ["Audit Log", "8.6"],
    ["Monitoring Agents", "8.6"],
    ["Durable Agents", "8.5", ["Limits across restarts"]],
    ["Secrets", "8.7", ["Secret fields", "Secret parameters", "Secret results", "reveal"]],
    ["Agent Use Cases", "8.3"],
    ["Agents in Practice", "8.4"],
  ]],
  ["tools", "Projects & Tools", "intermediate", [
    ["Modules", "10", ["use", "pub"]],
    ["Packages", "10.12"],
    ["Workspaces", "10.14"],
    ["Project Layout", "10.3"],
    ["Settings", "10.11"],
    ["Environment Variables", "10.13"],
    ["Editions", "10.8"],
    ["Limits and Sizes", "—"],
    ["The kavach Command", "—", ["new", "run", "build", "test", "check", "fix"]],
    ["Formatter", "—"],
    ["Editor Setup", "—"],
    ["Small Files", "12"],
    ["Tools for AI", "13", ["outline", "query", "context", "mcp"]],
  ]],
  ["speed", "Speed", "advanced", [
    ["How Kavach Is Fast", "—"],
    ["Loops on All Cores", "—"],
    ["Byte-packed Lists", "—"],
    ["Fast Sorting", "—"],
    ["As Data Grows", "—"],
    ["GPU", "15", ["gpu fn", "map on the GPU", "Records and two lists", "Same results as the CPU", "CPU or GPU", "Can I use it on the GPU?", "Sums in a loop", "Use cases"]],
    ["Native Executables", "—"],
  ]],
  ["other-languages", "Other Languages", "advanced", [
    ["Calling C", "—", ["Numbers", "Text", "Callbacks", "Records"]],
    ["A Library for Any Language", "—"],
    ["kavach audit", "—"],
  ]],
];

export type CourseSub = { slug: string; title: string };
export type CoursePage = { slug: string; title: string; spec: string; subs: CourseSub[] };
export type CoursePart = { n: number; slug: string; title: string; level: Level; pages: CoursePage[] };

export const course: CoursePart[] = raw.map(([slug, title, level, pages], n) => ({
  n,
  slug,
  title,
  level,
  pages: pages.map(([t, spec, subs = []]) => ({
    slug: slugOf(t),
    title: t,
    spec,
    subs: subs.map((s) => ({ slug: slugOf(s), title: s })),
  })),
}));
