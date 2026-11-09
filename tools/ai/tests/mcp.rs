//! The MCP server answers the protocol and runs the tools (spec 13.8).

use serde_json::{Value, json};
use std::path::Path;

/// Sends `requests` (one per line) and returns the replies.
fn exchange(requests: &[Value]) -> Vec<Value> {
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut output = Vec::new();
    kavach_ai::serve(input.as_bytes(), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn example(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .to_string_lossy()
        .to_string()
}

#[test]
fn speaks_the_protocol() {
    let replies = exchange(&[
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "nonsense"}),
    ]);
    assert_eq!(replies.len(), 3, "notifications get no reply");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "kavach");
    let tools: Vec<&str> = replies[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        tools,
        [
            "kavach_outline",
            "kavach_query",
            "kavach_context",
            "kavach_check",
            "kavach_run",
            "kavach_test"
        ]
    );
    assert_eq!(replies[2]["error"]["code"], -32601);
    assert_eq!(
        exchange(&[json!("not an object")]).len(),
        0,
        "a message without an id is a notification"
    );
}

#[test]
fn runs_the_tools() {
    let call = |name: &str, args: Value| {
        let reply = &exchange(&[
            json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": name, "arguments": args}}),
        ])[0];
        (
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .to_string(),
            reply["result"]["isError"].as_bool().unwrap(),
        )
    };
    let errors = example("03-errors/errors.kav");

    let (outline, failed) = call("kavach_outline", json!({"file": errors}));
    assert!(!failed && outline.contains("fn parse_age(text: String) -> Int ! AgeError { … }"));

    let (run, failed) = call("kavach_run", json!({"file": errors}));
    assert!(!failed && run.starts_with("exit code: 0") && run.contains("age = 31"));

    let (context, _) = call(
        "kavach_context",
        json!({"file": errors, "name": "age_next_year", "budget": 500}),
    );
    assert!(
        context.contains("const age = try parse_age(text)") && context.contains("type AgeError")
    );

    let (message, failed) = call("kavach_query", json!({"file": errors}));
    assert!(failed && message.contains("missing argument `kind`"));
}
