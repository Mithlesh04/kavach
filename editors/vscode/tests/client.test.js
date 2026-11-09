// Drives the real `kavach lsp` through client/rpc.js, as the extension
// does: an error comes back as a diagnostic, and hover, go to definition
// and completion answer. Plain Node, no packages:
//
//   node editors/vscode/tests/client.test.js target/debug/kavach

'use strict';

const assert = require('assert');
const os = require('os');
const path = require('path');
const { pathToFileURL } = require('url');
const { Connection } = require('../client/rpc');

const TEXT = `fn double(n: Int) -> Int {
    n * 2
}

fn main() {
    const x: Int = "text"
    print(double(4))
}
`;

async function main() {
  const c = new Connection(process.argv[2] || 'kavach');
  const published = new Promise((resolve) => c.on('textDocument/publishDiagnostics', resolve));
  const init = await c.request('initialize', { processId: process.pid, rootUri: null, capabilities: {} });
  assert.ok(init.capabilities.hoverProvider, 'hover');
  assert.ok(init.capabilities.definitionProvider, 'go to definition');
  c.notify('initialized', {});
  const uri = pathToFileURL(path.join(os.tmpdir(), 'kavach-client-test.kav')).href;
  c.notify('textDocument/didOpen', {
    textDocument: { uri, languageId: 'kavach', version: 1, text: TEXT },
  });
  const diagnostics = await published;
  assert.strictEqual(diagnostics.uri, uri);
  assert.strictEqual(diagnostics.diagnostics.length, 1, JSON.stringify(diagnostics));
  assert.strictEqual(diagnostics.diagnostics[0].range.start.line, 5);
  // `double` in `print(double(4))`: the call is an `Int`, and `double`
  // is declared on the first line.
  const at = { textDocument: { uri }, position: { line: 6, character: 11 } };
  const hover = await c.request('textDocument/hover', at);
  assert.ok(JSON.stringify(hover.contents).includes('Int'), JSON.stringify(hover));
  const found = await c.request('textDocument/definition', at);
  const place = Array.isArray(found) ? found[0] : found;
  assert.strictEqual(place.range.start.line, 0, JSON.stringify(found));
  // After `dou` in `print(dou)`, once typed.
  c.notify('textDocument/didChange', {
    textDocument: { uri, version: 2 },
    contentChanges: [{ text: TEXT.replace('print(double(4))', 'print(dou)') }],
  });
  const done = await c.request('textDocument/completion', {
    textDocument: { uri },
    position: { line: 6, character: 13 },
  });
  assert.deepStrictEqual(done.items.map((i) => i.label), ['double'], JSON.stringify(done));
  await c.stop();
  console.log('client test: ok');
}

setTimeout(() => {
  console.error('client test: no answer within 20 seconds');
  process.exit(1);
}, 20000).unref();

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
