// JSON-RPC with `kavach lsp` over its standard input and output, as the
// Language Server Protocol frames it (a `Content-Length` header, then the
// JSON). Plain Node, no packages and nothing from VS Code, so a test can
// drive the real server with it (tests/client.test.js).

'use strict';

const { spawn } = require('child_process');

class Connection {
  /** Starts `command lsp`. */
  constructor(command) {
    this.process = spawn(command, ['lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
    this.next = 1;
    this.waiting = new Map();
    this.handlers = new Map();
    this.buffer = Buffer.alloc(0);
    this.process.stdout.on('data', (data) => this.receive(data));
    this.process.on('exit', () => {
      for (const { reject } of this.waiting.values()) {
        reject(new Error('the Kavach language server stopped'));
      }
      this.waiting.clear();
    });
  }

  /** Sends a request; resolves with its result. */
  request(method, params) {
    const id = this.next++;
    this.send({ jsonrpc: '2.0', id, method, params });
    return new Promise((resolve, reject) => this.waiting.set(id, { resolve, reject }));
  }

  /** Sends a notification. */
  notify(method, params) {
    this.send({ jsonrpc: '2.0', method, params });
  }

  /** Calls `handler(params)` for each notification `method` from the server. */
  on(method, handler) {
    this.handlers.set(method, handler);
  }

  send(message) {
    const body = Buffer.from(JSON.stringify(message), 'utf8');
    this.process.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    this.process.stdin.write(body);
  }

  /** Reads whole messages out of what came in so far. */
  receive(data) {
    this.buffer = Buffer.concat([this.buffer, data]);
    for (;;) {
      const end = this.buffer.indexOf('\r\n\r\n');
      if (end < 0) return;
      const header = this.buffer.slice(0, end).toString('ascii');
      const match = /Content-Length: *(\d+)/i.exec(header);
      const length = match ? Number(match[1]) : 0;
      if (this.buffer.length < end + 4 + length) return;
      const body = this.buffer.slice(end + 4, end + 4 + length).toString('utf8');
      this.buffer = this.buffer.slice(end + 4 + length);
      this.dispatch(JSON.parse(body));
    }
  }

  dispatch(message) {
    if (message.id !== undefined && this.waiting.has(message.id)) {
      const { resolve, reject } = this.waiting.get(message.id);
      this.waiting.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
      return;
    }
    const handler = this.handlers.get(message.method);
    if (handler) handler(message.params);
  }

  /** Asks the server to stop, then ends it. */
  async stop() {
    try {
      await this.request('shutdown', null);
      this.notify('exit', null);
    } catch (_) {
      this.process.kill();
    }
  }
}

module.exports = { Connection };
