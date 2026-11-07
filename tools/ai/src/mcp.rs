//! `kavach mcp` (spec 13.8): a Model Context Protocol server on stdin and
//! stdout, so any AI assistant can call the Kavach tools directly. It runs
//! locally; code never leaves the machine.

use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// A tool parameter: name, description, required.
type Param = (&'static str, &'static str, bool);
/// A tool: name, description, parameters.
type Tool = (&'static str, &'static str, &'static [Param]);

const TOOLS: &[Tool] = &[
    (
        "kavach_outline",
        "Declarations of a .kav file without function bodies: signatures, contracts, examples, doc comments. Use before reading a whole file.",
        &[("file", "Path to a .kav file", true)],
    ),
    (
        "kavach_query",
        "Exact answers from the compiler: callers of a name, functions that use a capability, functions that fail with an error type, a type's definition, or declarations whose name contains text.",
        &[
            ("kind", "callers, uses, fails, type or find", true),
            ("name", "The name or text to look for", true),
            ("file", "Path to a .kav file", true),
        ],
    ),
    (
        "kavach_context",
        "The full code of one function, method (Type.method) or type, plus the outlines of everything it uses, cut to a token budget.",
        &[
            ("name", "What to get context for", true),
            ("file", "Path to a .kav file", true),
            ("budget", "Token budget (default 2000)", false),
        ],
    ),
    (
        "kavach_check",
        "Compile errors and warnings, one JSON line each. Empty when the file is fine.",
        &[("file", "Path to a .kav file", true)],
    ),
    (
        "kavach_run",
        "Run a .kav file and return its output and exit code.",
        &[("file", "Path to a .kav file", true)],
    ),
    (
        "kavach_test",
        "Run a file's `example` clauses and `test` blocks.",
        &[("file", "Path to a .kav file", true)],
    ),
];

/// Serves MCP requests, one JSON message per line, until `input` ends.
pub fn serve(input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle(&request),
            Err(e) => Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}),
            ),
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The reply to one message; `None` for notifications.
fn handle(request: &Value) -> Option<Value> {
    let id = request.get("id")?.clone();
    let method = request["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "kavach", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(
            json!({"tools": TOOLS.iter().map(|(name, description, params)| tool_schema(name, description, params)).collect::<Vec<_>>()}),
        ),
        "tools/call" => Ok(call(
            &request["params"]["name"],
            &request["params"]["arguments"],
        )),
        _ => Err(json!({"code": -32601, "message": format!("unknown method `{method}`")})),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
    })
}

fn tool_schema(name: &str, description: &str, params: &[(&str, &str, bool)]) -> Value {
    let properties: serde_json::Map<String, Value> = params
        .iter()
        .map(|(p, d, _)| (p.to_string(), json!({"type": if *p == "budget" { "integer" } else { "string" }, "description": d})))
        .collect();
    let required: Vec<&str> = params
        .iter()
        .filter(|(_, _, r)| *r)
        .map(|(p, _, _)| *p)
        .collect();
    json!({"name": name, "description": description, "inputSchema": {"type": "object", "properties": properties, "required": required}})
}

/// Runs a tool; failures are reported as tool errors, not protocol errors.
fn call(name: &Value, args: &Value) -> Value {
    let text = |key: &str| {
        args[key]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("missing argument `{key}`"))
    };
    let outcome = (|| -> Result<String, String> {
        let path = text("file")?;
        let src = std::fs::read_to_string(&path).map_err(|e| format!("can't read {path}: {e}"))?;
        let c = kavach_driver::compile(&src, &path, kavach_driver::Options::default());
        let needs_clean = |c: &kavach_driver::Compiled| {
            if c.has_errors() {
                Err(c.render())
            } else {
                Ok(())
            }
        };
        match name.as_str().unwrap_or("") {
            "kavach_check" => Ok(crate::messages_json(&c)),
            "kavach_outline" => needs_clean(&c).map(|_| crate::outline(&c)),
            "kavach_query" => {
                needs_clean(&c)?;
                crate::query(
                    &crate::Index::build(&c, &path),
                    &text("kind")?,
                    &text("name")?,
                )
            }
            "kavach_context" => {
                needs_clean(&c)?;
                let budget = args["budget"].as_u64().unwrap_or(2000) as usize;
                crate::context(&crate::Index::build(&c, &path), &text("name")?, budget)
            }
            "kavach_run" | "kavach_test" => {
                needs_clean(&c)?;
                let out = if name == "kavach_run" {
                    kavach_interp::run(&c)
                } else {
                    kavach_interp::test(&c, &Default::default())
                };
                Ok(format!(
                    "exit code: {}\n--- stdout\n{}--- stderr\n{}",
                    out.exit_code, out.stdout, out.stderr
                ))
            }
            other => Err(format!("unknown tool `{other}`")),
        }
    })();
    match outcome {
        Ok(text) => json!({"content": [{"type": "text", "text": text}], "isError": false}),
        Err(text) => json!({"content": [{"type": "text", "text": text}], "isError": true}),
    }
}
