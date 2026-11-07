// The VS Code side of the Kavach language server (`kavach lsp`, spec 10.4):
// errors as you type, hover, go to definition and completion for `.kav`
// files. The
// messages go through `client/rpc.js`; nothing else is needed, so the
// extension installs without any npm packages.

'use strict';

const vscode = require('vscode');
const { Connection } = require('./client/rpc');

let connection;

/** An LSP range as a VS Code `Range`. */
function range(r) {
  return new vscode.Range(r.start.line, r.start.character, r.end.line, r.end.character);
}

function position(document, at) {
  return {
    textDocument: { uri: document.uri.toString() },
    position: { line: at.line, character: at.character },
  };
}

const isKavach = (document) => document.languageId === 'kavach';

async function activate(context) {
  const command = vscode.workspace.getConfiguration('kavach').get('path') || 'kavach';
  connection = new Connection(command);
  connection.process.on('error', (e) => {
    vscode.window.showWarningMessage(
      `Kavach: can't start \`${command} lsp\` (${e.message}). Set "kavach.path" to the kavach program.`,
    );
  });
  const diagnostics = vscode.languages.createDiagnosticCollection('kavach');
  context.subscriptions.push(diagnostics);
  connection.on('textDocument/publishDiagnostics', (params) => {
    const found = params.diagnostics.map((d) => {
      const severity = d.severity === 2
        ? vscode.DiagnosticSeverity.Warning
        : vscode.DiagnosticSeverity.Error;
      const diagnostic = new vscode.Diagnostic(range(d.range), d.message, severity);
      diagnostic.source = 'kavach';
      return diagnostic;
    });
    diagnostics.set(vscode.Uri.parse(params.uri), found);
  });
  await connection.request('initialize', {
    processId: process.pid,
    rootUri: null,
    capabilities: {},
  });
  connection.notify('initialized', {});

  const open = (document) => {
    if (!isKavach(document)) return;
    connection.notify('textDocument/didOpen', {
      textDocument: {
        uri: document.uri.toString(),
        languageId: 'kavach',
        version: document.version,
        text: document.getText(),
      },
    });
  };
  vscode.workspace.textDocuments.forEach(open);
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(open),
    // The whole text each time: simple, and checks are fast.
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (!isKavach(e.document)) return;
      connection.notify('textDocument/didChange', {
        textDocument: { uri: e.document.uri.toString(), version: e.document.version },
        contentChanges: [{ text: e.document.getText() }],
      });
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      if (!isKavach(document)) return;
      connection.notify('textDocument/didClose', {
        textDocument: { uri: document.uri.toString() },
      });
    }),
    vscode.languages.registerHoverProvider('kavach', {
      async provideHover(document, at) {
        const hover = await connection.request('textDocument/hover', position(document, at));
        if (!hover || !hover.contents) return null;
        const text = typeof hover.contents === 'string' ? hover.contents : hover.contents.value;
        return new vscode.Hover(new vscode.MarkdownString(text));
      },
    }),
    vscode.languages.registerCompletionItemProvider('kavach', {
      async provideCompletionItems(document, at) {
        const found = await connection.request('textDocument/completion', position(document, at));
        if (!found) return [];
        // LSP numbers the kinds from 1, VS Code from 0.
        return found.items.map((i) => new vscode.CompletionItem(i.label, i.kind - 1));
      },
    }, '.'),
    vscode.languages.registerDefinitionProvider('kavach', {
      async provideDefinition(document, at) {
        const found = await connection.request('textDocument/definition', position(document, at));
        if (!found) return null;
        const places = Array.isArray(found) ? found : [found];
        return places.map((p) => new vscode.Location(vscode.Uri.parse(p.uri), range(p.range)));
      },
    }),
  );
}

function deactivate() {
  return connection ? connection.stop() : undefined;
}

module.exports = { activate, deactivate };
