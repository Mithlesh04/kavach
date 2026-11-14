"""A small Model Context Protocol client over stdio, for the `tools`
condition: it starts a language's MCP server (`kavach mcp`), lists its
tools for the model, and calls them. One JSON-RPC message per line."""

import json
import subprocess


class Server:
    def __init__(self, command):
        self.proc = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, text=True, encoding="utf-8")
        self.next_id = 0
        self.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                    "clientInfo": {"name": "ai-writes", "version": "1"}})

    def request(self, method, params):
        self.next_id += 1
        message = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()
        while True:
            line = self.proc.stdout.readline()
            if not line:
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
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        text = "".join(c.get("text", "") for c in result.get("content", []))
        return text, bool(result.get("isError"))

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=10)
