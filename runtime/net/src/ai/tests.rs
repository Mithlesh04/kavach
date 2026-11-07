use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

fn request() -> Request {
    let settings = Tree::Object(vec![
        ("model".into(), Tree::Text("test-model".into())),
        ("max_tokens".into(), Tree::Int(64)),
    ]);
    let schema = Tree::Object(vec![("type".into(), Tree::Text("object".into()))]);
    Request::new(&settings, "Refund order 7?".into(), schema, true)
}

#[test]
fn the_body_asks_for_the_schema() {
    let tree = json::parse(&body(&request()), &Limits::default()).unwrap();
    assert!(matches!(field(&tree, "model"), Some(Tree::Text(m)) if m == "test-model"));
    assert!(matches!(field(&tree, "max_tokens"), Some(Tree::Int(64))));
    let format = field(field(&tree, "output_config").unwrap(), "format").unwrap();
    assert!(matches!(field(format, "type"), Some(Tree::Text(t)) if t == "json_schema"));
    assert!(field(format, "schema").is_some());
    // No forced tool call: newer models refuse one.
    assert!(field(&tree, "tools").is_none() && field(&tree, "tool_choice").is_none());
}

#[test]
fn the_answer_is_the_first_text_block() {
    let reply = r#"{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"{\"approve\":true}"}]}"#;
    let tree = json::parse(reply, &Limits::default()).unwrap();
    assert_eq!(answer_text(&tree).as_deref(), Some(r#"{"approve":true}"#));
    let error = json::parse(
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad model"}}"#,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(error_message(&error).as_deref(), Some("bad model"));
}

#[test]
fn chunked_bodies_are_joined() {
    assert_eq!(
        http::dechunk(b"4\r\nWiki\r\n5;x=1\r\npedia\r\n0\r\n\r\n").unwrap(),
        b"Wikipedia"
    );
    assert!(http::dechunk(b"9\r\nshort\r\n").is_err());
    let (status, body) =
        http::parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\n\r\n")
            .unwrap();
    assert_eq!((status, body.as_slice()), (200, &b"hi"[..]));
}

/// A server that answers one request with `reply`, and gives back what it
/// was sent.
fn fake_server(reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        // Read the head and the body announced by content-length.
        loop {
            let n = s.read(&mut buf).unwrap();
            got.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&got);
            if let Some(end) = text.find("\r\n\r\n") {
                let length = text[..end]
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if got.len() >= end + 4 + length {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        s.write_all(reply.as_bytes()).unwrap();
        String::from_utf8_lossy(&got).into_owned()
    });
    (format!("http://{address}"), handle)
}

#[test]
fn a_call_reaches_the_server_and_reads_the_answer() {
    let (base, server) = fake_server(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n\
         {\"content\":[{\"type\":\"text\",\"text\":\"{\\\"approve\\\":false}\"}],\"stop_reason\":\"end_turn\"}",
    );
    let outcome = call(&base, "sk-test", &request());
    let sent = server.join().unwrap();
    assert!(sent.starts_with("POST /v1/messages HTTP/1.1"));
    assert!(sent.contains("x-api-key: sk-test"));
    assert!(sent.contains("anthropic-version: 2023-06-01"));
    match outcome {
        Outcome::Answer(t, _) => assert!(matches!(field(&t, "approve"), Some(Tree::Bool(false)))),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_refusal_gives_its_status_and_message() {
    let (base, server) = fake_server(
        "HTTP/1.1 401 Unauthorized\r\nconnection: close\r\n\r\n\
         {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid x-api-key\"}}",
    );
    let outcome = call(&base, "wrong", &request());
    server.join().unwrap();
    match outcome {
        Outcome::Failed { status, message } => {
            assert_eq!((status, message.as_str()), (401, "invalid x-api-key"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_refusal_is_not_an_answer() {
    let (base, server) = fake_server(
        "HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n\
         {\"content\":[],\"stop_reason\":\"refusal\"}",
    );
    let outcome = call(&base, "sk-test", &request());
    server.join().unwrap();
    match outcome {
        Outcome::Failed { status, message } => {
            assert_eq!(
                (status, message.as_str()),
                (200, "the model refused to answer")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_answer_cut_off_is_not_asked_again() {
    let (base, server) = fake_server(
        "HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n\
         {\"content\":[{\"type\":\"text\",\"text\":\"{\\\"appr\"}],\"stop_reason\":\"max_tokens\"}",
    );
    let outcome = call(&base, "sk-test", &request());
    server.join().unwrap();
    assert!(
        matches!(&outcome, Outcome::Failed { status: 200, message } if message.contains("max_tokens (64)")),
        "{outcome:?}"
    );
}
