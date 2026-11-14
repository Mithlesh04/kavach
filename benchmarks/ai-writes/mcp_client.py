"""A small Model Context Protocol client over stdio, for the `tools`
condition: it starts a language's MCP server (`kavach mcp`), lists its
tools for the model, and calls them. One JSON-RPC message per line.

A call that takes longer than `LIMIT` (a model's code that never ends:
`kavach mcp` runs code with no time limit of its own) stops the server,
starts a new one, and answers the model that the run was stopped."""

import json
import queue
import subprocess
import threading

LIMIT = 60


class Server:
    def __init__(self, command):
        self.command = command
        self.start()

    def start(self):
        self.proc = subprocess.Popen(self.command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, text=True, encoding="utf-8")
        self.lines = queue.Queue()
        threading.Thread(target=self.read, args=(self.proc, self.lines), daemon=True).start()
        self.next_id = 0
        self.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                    "clientInfo": {"name": "ai-writes", "version": "1"}})

    @staticmethod
    def read(proc, lines):
        for line in proc.stdout:
            lines.put(line)
        lines.put(None)

    def request(self, method, params):
        self.next_id += 1
        message = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()
        while True:
            line = self.lines.get(timeout=LIMIT)
            if line is None:
                raise RuntimeError("the MCP server stopped")
            reply = json.loads(line)
            if reply.get("id") == self.next_id:
                return reply.get("result", {"isError": True, "content": [{"type": "text", "text": str(reply.get("error"))}]})

    def tools(self):
        """The server's tools, in the shape the Messages API takes."""
        listed = self.request("tools/list", {}).get("tools", [])
        return [{"name": t["name"], "description": t["description"], "input_schema": t["inputSchema"]}
                for t in listed]

    def call(self, name, arguments):
        """A tool's text result, and whether it is an error."""
        try:
            result = self.request("tools/call", {"name": name, "arguments": arguments})
        except (queue.Empty, RuntimeError, OSError) as e:
            stopped = isinstance(e, queue.Empty)
            self.restart()
            if stopped:
                return f"stopped: it ran longer than {LIMIT} seconds (a loop that never ends?)", True
            return f"the tool failed: {e}", True
        text = "".join(c.get("text", "") for c in result.get("content", []))
        return text, bool(result.get("isError"))

    def restart(self):
        self.proc.kill()
        self.proc.wait(timeout=10)
        self.start()

    def close(self):
        try:
            self.proc.stdin.close()
            self.proc.wait(timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            self.proc.kill()
