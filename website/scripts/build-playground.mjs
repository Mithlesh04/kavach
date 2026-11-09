// Builds the playground engine (decision 0078) for the browser and puts
// it where the site serves it: public/playground/engine.wasm. The folder
// is git-ignored: like every build output, the engine is built from
// source, never committed.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = path.join(process.cwd(), "..");
try {
  execFileSync(
    "cargo",
    ["build", "-p", "kavach-playground", "--release", "--target", "wasm32-wasip1"],
    { cwd: root, stdio: "inherit" },
  );
} catch {
  console.error(
    "building the playground engine failed — is the target installed?\n" +
      "  rustup target add wasm32-wasip1",
  );
  process.exit(1);
}

const from = path.join(root, "target", "wasm32-wasip1", "release", "kavach-playground.wasm");
const to = path.join(process.cwd(), "public", "playground", "engine.wasm");
fs.mkdirSync(path.dirname(to), { recursive: true });
fs.copyFileSync(from, to);

// The editor colors code in the browser with the repository's own grammar,
// the same file the VS Code extension and the build-time highlighter use.
fs.copyFileSync(
  path.join(root, "editors", "vscode", "syntaxes", "kavach.tmLanguage.json"),
  path.join(process.cwd(), "public", "playground", "kavach.tmLanguage.json"),
);
const size = (fs.statSync(to).size / 1024 / 1024).toFixed(1);
console.log(`playground engine: ${size} MB + grammar -> public/playground/`);
