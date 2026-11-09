//! The playground's engine (decision 0078): reads a Kavach program on
//! standard input, checks it with the same pipeline every tool uses, and
//! writes the messages as JSON on standard output. The docs site builds
//! it for `wasm32-wasip1` and runs it in the reader's browser; natively
//! it works the same, so it can be tested with `cargo test`.

use kavach_driver::{Options, Severity, compile, render};
use kavach_runtime_core::json::{Tree, write};
use std::io::Read;

fn main() {
    let mut src = String::new();
    if std::io::stdin().read_to_string(&mut src).is_err() {
        print!("{}", write(&answer(Err("the input isn't UTF-8 text")), 0));
        return;
    }
    print!("{}", write(&answer(Ok(&src)), 0));
}

/// The engine's whole answer: the messages, or why there are none.
fn answer(src: Result<&str, &str>) -> Tree {
    match src {
        Err(why) => Tree::Object(vec![("failed".to_string(), Tree::Text(why.to_string()))]),
        Ok(src) => {
            let compiled = compile(src, "playground.kav", Options::default());
            let messages = compiled.messages.iter().map(|m| message(src, m)).collect();
            Tree::Object(vec![("messages".to_string(), Tree::Array(messages))])
        }
    }
}

/// One message: its severity, its byte range in the source, its text,
/// and the full rendering the CLI would print.
fn message(src: &str, m: &kavach_driver::Message) -> Tree {
    let severity = match m.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    Tree::Object(vec![
        ("severity".to_string(), Tree::Text(severity.to_string())),
        ("start".to_string(), Tree::Int(i64::from(m.span.start))),
        ("end".to_string(), Tree::Int(i64::from(m.span.end))),
        ("text".to_string(), Tree::Text(m.text.clone())),
        (
            "rendered".to_string(),
            Tree::Text(render(src, "playground.kav", m)),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_program_has_no_messages() {
        let json = write(&answer(Ok("print(\"hi\")\n")), 0);
        assert_eq!(json, r#"{"messages":[]}"#);
    }

    #[test]
    fn an_error_is_reported_with_its_place() {
        let src = "print(missing)\n";
        let json = write(&answer(Ok(src)), 0);
        assert!(json.contains(r#""severity":"error""#), "{json}");
        assert!(json.contains(r#""start":"#), "{json}");
    }
}
