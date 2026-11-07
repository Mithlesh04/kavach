use super::*;

/// Runs the server on `messages` and returns everything it sent, and
/// whether it was asked to shut down before it stopped.
fn run(messages: &[Value]) -> (Vec<Value>, bool) {
    let mut input = Vec::new();
    for m in messages {
        rpc::write(&mut input, m).unwrap();
    }
    let mut output = Vec::new();
    let clean = serve(&input[..], &mut output).unwrap();
    let mut sent = Vec::new();
    let mut rest = &output[..];
    while let Some(m) = rpc::read(&mut rest).unwrap() {
        sent.push(m.unwrap());
    }
    (sent, clean)
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn notify(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

const URI: &str = "file:///kavach-lsp-test/no-such-folder/main.kav";

fn open(text: &str) -> Value {
    notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "kavach", "version": 1, "text": text}}),
    )
}

fn change(changes: Value) -> Value {
    notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2}, "contentChanges": changes}),
    )
}

/// The diagnostics of each `publishDiagnostics` sent, in order.
fn published(sent: &[Value]) -> Vec<Vec<Value>> {
    sent.iter()
        .filter(|m| m["method"] == "textDocument/publishDiagnostics")
        .map(|m| {
            assert_eq!(m["params"]["uri"], URI);
            m["params"]["diagnostics"].as_array().unwrap().clone()
        })
        .collect()
}

#[test]
fn initialize_shutdown_exit() {
    let (sent, clean) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        notify("initialized", json!({})),
        request(2, "shutdown", Value::Null),
        notify("exit", Value::Null),
        request(3, "never-read", Value::Null),
    ]);
    assert!(clean);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0]["id"], 1);
    assert_eq!(
        sent[0]["result"]["capabilities"]["textDocumentSync"]["change"],
        2
    );
    assert_eq!(sent[1], json!({"jsonrpc": "2.0", "id": 2, "result": null}));
}

#[test]
fn errors_for_bad_requests() {
    let (sent, clean) = run(&[
        request(1, "textDocument/hover", json!({})),
        request(2, "initialize", json!({})),
        request(3, "no/such/method", json!({})),
        request(4, "shutdown", Value::Null),
        request(5, "initialize", json!({})),
    ]);
    assert!(clean, "it was asked to shut down, then the input ended");
    let codes: Vec<&Value> = sent.iter().map(|m| &m["error"]["code"]).collect();
    assert_eq!(
        codes,
        [
            &json!(-32002),
            &Value::Null,
            &json!(-32601),
            &Value::Null,
            &json!(-32600)
        ]
    );
}

#[test]
fn bad_json_gets_a_parse_error() {
    let mut input = b"Content-Length: 5\r\n\r\n{oops".to_vec();
    rpc::write(&mut input, &notify("exit", Value::Null)).unwrap();
    let mut output = Vec::new();
    assert!(
        !serve(&input[..], &mut output).unwrap(),
        "exit without shutdown"
    );
    let reply = rpc::read(&mut &output[..]).unwrap().unwrap().unwrap();
    assert_eq!(reply["error"]["code"], -32700);
}

#[test]
fn diagnostics_follow_the_text() {
    let (sent, _) = run(&[
        request(1, "initialize", json!({})),
        open("const total = 1\nprint(totl)\n"),
        // Incremental: `totl` → `total`.
        change(json!([{"range": {"start": {"line": 1, "character": 9},
                                 "end": {"line": 1, "character": 9}}, "text": "a"}])),
        // Whole text.
        change(json!([{"text": "print(\"é😀\" + x)\n"}])),
        notify(
            "textDocument/didClose",
            json!({"textDocument": {"uri": URI}}),
        ),
    ]);
    let lists = published(&sent);
    assert_eq!(lists.len(), 4);
    let first = &lists[0];
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(first[0]["severity"], 1);
    assert_eq!(first[0]["source"], "kavach");
    assert!(first[0]["message"].as_str().unwrap().contains("totl"));
    assert_eq!(
        first[0]["range"],
        json!({"start": {"line": 1, "character": 6}, "end": {"line": 1, "character": 10}})
    );
    assert_eq!(lists[1], Vec::<Value>::new(), "fixed");
    // `x` comes after `"é😀"`: `é` is one UTF-16 unit and `😀` two.
    let x = lists[2]
        .iter()
        .find(|d| d["message"].as_str().unwrap().contains("`x`"));
    assert_eq!(
        x.unwrap()["range"]["start"],
        json!({"line": 0, "character": 14})
    );
    assert_eq!(lists[3], Vec::<Value>::new(), "closing clears them");
}

#[test]
fn an_open_string_is_shown_in_the_file() {
    let (sent, _) = run(&[request(1, "initialize", json!({})), open("print(\"hi)\n")]);
    let lists = published(&sent);
    assert_eq!(lists.len(), 1);
    assert!(
        !lists[0].is_empty(),
        "the error must not be lost in the standard library"
    );
    for d in &lists[0] {
        assert!(d["range"]["start"]["line"].as_u64().unwrap() <= 1, "{d}");
    }
}

#[test]
fn a_project_is_checked_as_a_whole() {
    let dir = std::env::temp_dir().join(format!("kavach-lsp-{}", std::process::id()));
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"demo\"\n").unwrap();
    let main_text = "use orders\n\nfn main() {\n    print(orders.count())\n}\n";
    std::fs::write(src.join("main.kav"), main_text).unwrap();
    std::fs::write(
        src.join("orders.kav"),
        "pub fn count() -> Int {\n    return 1\n}\n",
    )
    .unwrap();
    let main = uri::from_path(&src.join("main.kav").to_string_lossy());
    let orders = uri::from_path(&src.join("orders.kav").to_string_lossy());
    let open = |uri: &str, text: &str| {
        notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "text": text}}),
        )
    };
    let (sent, _) = run(&[
        request(1, "initialize", json!({})),
        open(&main, main_text),
        // Unsaved: `count` returns a String now.
        open(&orders, "pub fn count() -> Int {\n    return \"one\"\n}\n"),
    ]);
    std::fs::remove_dir_all(&dir).unwrap();
    let by_uri = |uri: &str| -> Vec<usize> {
        sent.iter()
            .filter(|m| m["params"]["uri"] == uri)
            .map(|m| m["params"]["diagnostics"].as_array().unwrap().len())
            .collect()
    };
    // Opening main.kav checks orders.kav from the disk: no errors. Opening
    // orders.kav checks its unsaved text, and main.kav with it.
    assert_eq!(by_uri(&main), [0, 0]);
    assert_eq!(by_uri(&orders), [0, 1]);
}

#[test]
fn positions() {
    let text = "ab\né😀x\n";
    // (byte offset, line, character)
    let table = [
        (0, 0, 0),
        (2, 0, 2),
        (3, 1, 0),
        (5, 1, 1),
        (9, 1, 3),
        (10, 1, 4),
        (11, 2, 0),
    ];
    for (offset, line, character) in table {
        let p = json!({"line": line, "character": character});
        assert_eq!(position::position(text, offset), p, "offset {offset}");
        assert_eq!(position::offset(text, &p), offset, "{p}");
    }
    // Inside a character, past the end of a line and past the end.
    assert_eq!(
        position::position(text, 6),
        json!({"line": 1, "character": 1})
    );
    assert_eq!(
        position::offset(text, &json!({"line": 0, "character": 9})),
        2
    );
    assert_eq!(
        position::offset(text, &json!({"line": 7, "character": 0})),
        text.len()
    );
}

#[test]
fn uris() {
    let table = [
        ("file:///C:/work/a%20b.kav", "C:/work/a b.kav"),
        ("file:///home/me/main.kav", "/home/me/main.kav"),
        ("file://server/share/x.kav", "//server/share/x.kav"),
    ];
    for (uri, path) in table {
        assert_eq!(uri::to_path(uri).as_deref(), Some(path));
        assert_eq!(uri::from_path(path), uri);
    }
    assert_eq!(
        uri::to_path("file:///d%3A/x.kav").as_deref(),
        Some("d:/x.kav")
    );
    assert_eq!(uri::from_path(r"D:\x\y.kav"), "file:///D:/x/y.kav");
    assert_eq!(uri::to_path("untitled:Untitled-1"), None);
    assert_eq!(uri::to_path("file:///bad%zz"), None);
}

const PROGRAM: &str = "fn double(n: Int) -> Int {\n    n * 2\n}\n\nfn main() {\n    let total = double(21)\n    print(total)\n}\n";

fn at(id: u64, method: &str, line: u64, character: u64) -> Value {
    request(
        id,
        method,
        json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": character}}),
    )
}

#[test]
fn hover_shows_the_type_under_the_cursor() {
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(PROGRAM),
        // `total` in `print(total)`.
        at(2, "textDocument/hover", 6, 12),
        // Between functions: nothing.
        at(3, "textDocument/hover", 3, 0),
    ]);
    assert_eq!(sent[0]["result"]["capabilities"]["hoverProvider"], true);
    let hover = sent.iter().find(|m| m["id"] == 2).unwrap();
    assert_eq!(
        hover["result"]["contents"]["value"],
        "```kavach\ntotal: Int\n```"
    );
    assert_eq!(
        hover["result"]["range"],
        json!({"start": {"line": 6, "character": 10}, "end": {"line": 6, "character": 15}})
    );
    let nothing = sent.iter().find(|m| m["id"] == 3).unwrap();
    assert_eq!(nothing["result"], Value::Null);
}

#[test]
fn definition_goes_to_where_a_name_is_declared() {
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(PROGRAM),
        // `double` in `double(21)`, and `total` in `print(total)`.
        at(2, "textDocument/definition", 5, 17),
        at(3, "textDocument/definition", 6, 12),
        // `print` is built in: no place to go.
        at(4, "textDocument/definition", 6, 5),
    ]);
    assert_eq!(
        sent[0]["result"]["capabilities"]["definitionProvider"],
        true
    );
    let result = |id: u64| sent.iter().find(|m| m["id"] == id).unwrap()["result"].clone();
    assert_eq!(result(2)["uri"], URI);
    assert_eq!(
        result(2)["range"]["start"],
        json!({"line": 0, "character": 3})
    );
    assert_eq!(
        result(3)["range"]["start"],
        json!({"line": 5, "character": 8})
    );
    assert_eq!(result(4), Value::Null);
}

#[test]
fn definition_reaches_another_file_of_the_project() {
    let dir = std::env::temp_dir().join(format!("kavach-lsp-def-{}", std::process::id()));
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"demo\"\n").unwrap();
    let main_text = "use orders\n\nfn main() {\n    print(orders.count())\n}\n";
    std::fs::write(src.join("main.kav"), main_text).unwrap();
    std::fs::write(
        src.join("orders.kav"),
        "pub fn count() -> Int {\n    1\n}\n",
    )
    .unwrap();
    let main = uri::from_path(&src.join("main.kav").to_string_lossy());
    let open = notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": main, "text": main_text}}),
    );
    // `count` in `orders.count()`.
    let ask = request(
        2,
        "textDocument/definition",
        json!({"textDocument": {"uri": main}, "position": {"line": 3, "character": 18}}),
    );
    let (sent, _) = run(&[request(1, "initialize", json!({})), open, ask]);
    let orders = uri::from_path(&src.join("orders.kav").to_string_lossy());
    std::fs::remove_dir_all(&dir).unwrap();
    let found = sent.iter().find(|m| m["id"] == 2).unwrap();
    assert_eq!(found["result"]["uri"], orders);
    assert_eq!(
        found["result"]["range"]["start"],
        json!({"line": 0, "character": 7})
    );
}

const ORDERS: &str = "type Order {\n    id: Int\n    paid: Bool\n\n    fn label(self) -> String {\n        \"order {self.id}\"\n    }\n}\n\nfn double(n: Int) -> Int {\n    n * 2\n}\n\nfn main() {\n    const order = Order(id: 1, paid: true)\n    let total = double(21)\n    print(do)\n}\n";

/// The labels a completion answered with.
fn labels(sent: &[Value], id: u64) -> Vec<String> {
    let answer = sent.iter().find(|m| m["id"] == id).unwrap();
    let items = answer["result"]["items"].as_array().unwrap();
    items
        .iter()
        .map(|i| i["label"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn completion_offers_the_names_in_scope() {
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(ORDERS),
        // After `do` in `print(do)`.
        at(2, "textDocument/completion", 16, 12),
        // After `print(`: everything in scope, locals too.
        at(3, "textDocument/completion", 16, 10),
    ]);
    let completion = &sent[0]["result"]["capabilities"]["completionProvider"];
    assert_eq!(completion["triggerCharacters"], json!(["."]));
    assert_eq!(labels(&sent, 2), ["double"]);
    let all = labels(&sent, 3);
    for name in ["order", "total", "double", "main", "Order", "print"] {
        assert!(all.iter().any(|l| l == name), "{name} in {all:?}");
    }
    // A method is reached through a value, not by name.
    assert!(!all.iter().any(|l| l == "label"), "{all:?}");
}

#[test]
fn completion_after_a_dot_offers_fields_and_methods() {
    let text = ORDERS.replace("print(do)", "print(order.)\n    print(total.ab)");
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(&text),
        // After `order.`, and after `total.ab`.
        at(2, "textDocument/completion", 16, 16),
        at(3, "textDocument/completion", 17, 18),
    ]);
    assert_eq!(labels(&sent, 2), ["id", "paid", "label"]);
    assert_eq!(labels(&sent, 3), ["abs"]);
}

/// Semantic tokens as (line, column, length, type, modifiers), decoded
/// from the protocol's relative numbers.
fn decoded(data: &Value) -> Vec<(u64, u64, u64, &'static str, u64)> {
    let numbers: Vec<u64> = data
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap())
        .collect();
    let (mut line, mut col) = (0, 0);
    numbers
        .chunks(5)
        .map(|t| {
            line += t[0];
            col = if t[0] == 0 { col + t[1] } else { t[1] };
            (line, col, t[2], tokens::TYPES[t[3] as usize], t[4])
        })
        .collect()
}

#[test]
fn semantic_tokens_mark_imported_names_and_built_in_methods() {
    let program = "use std.math\n\ntype Bag {\n    items: [Int]\n    fn count(self) -> Int {\n        self.items.len()\n    }\n}\n\nfn main() {\n    let xs = [1]\n    xs.push(2)\n    const b = Bag(items: xs)\n    print(b.count())\n    print(math.sqrt(4.0))\n}\n";
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(program),
        request(
            2,
            "textDocument/semanticTokens/full",
            json!({"textDocument": {"uri": URI}}),
        ),
    ]);
    let legend = &sent[0]["result"]["capabilities"]["semanticTokensProvider"]["legend"];
    assert_eq!(
        legend["tokenModifiers"],
        json!(["imported", "defaultLibrary"])
    );
    let reply = sent.iter().find(|m| m["id"] == 2).unwrap();
    let got = decoded(&reply["result"]["data"]);
    // Built-in methods are `defaultLibrary` (2); the program's own method
    // is a plain method; a module and what is used through it, imported (1).
    let want = [
        (5, 19, 3, "method", 2),
        (11, 7, 4, "method", 2),
        (13, 12, 5, "method", 0),
        (14, 10, 4, "namespace", 1),
        (14, 15, 4, "function", 1),
    ];
    assert_eq!(got, want);
}

#[test]
fn a_quick_fix_marks_a_fitting_fn_as_a_gpu_fn() {
    let program = "fn helper(x: Float32) -> Float32 {\n    x * 2.0\n}\n\ngpu fn run(x: Float32) -> Float32 {\n    helper(x)\n}\n\nfn main() {}\n";
    let diagnostic = json!({
        "range": {"start": {"line": 5, "character": 4}, "end": {"line": 5, "character": 13}},
        "message": "a `gpu fn` can't use `helper`: it isn't a `gpu fn`, but it fits the GPU's rules. Mark it `gpu fn helper`; it still runs on the CPU when called there (spec 15.1)",
    });
    let (sent, _) = run(&[
        request(1, "initialize", json!({"capabilities": {}})),
        open(program),
        request(
            2,
            "textDocument/codeAction",
            json!({"textDocument": {"uri": URI}, "range": diagnostic["range"], "context": {"diagnostics": [diagnostic]}}),
        ),
    ]);
    let reply = sent.iter().find(|m| m["id"] == 2).unwrap();
    let edit = &reply["result"][0]["edit"]["changes"][URI][0];
    assert_eq!(edit["newText"], "gpu ");
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 0}));
    assert_eq!(reply["result"][0]["kind"], "quickfix");
}
