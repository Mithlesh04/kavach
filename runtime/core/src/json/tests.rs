use super::*;

fn ok(text: &str) -> Tree {
    parse(text, &Limits::default()).unwrap_or_else(|e| panic!("{text}: {e:?}"))
}

fn err(text: &str) -> Error {
    parse(text, &Limits::default()).expect_err(text)
}

fn syntax_at(e: &Error) -> (usize, usize) {
    match e {
        Error::Syntax { line, column, .. } => (*line, *column),
        other => panic!("not a syntax error: {other:?}"),
    }
}

#[test]
fn values_parse_and_write_back_the_same() {
    for text in [
        "null",
        "true",
        "[1,-2,3.5,1e21,\"a\"]",
        "{\"name\":\"Asha\",\"tags\":[],\"meta\":{},\"n\":null}",
        "\"tab\\tquote\\\"slash\\\\\\u0001\"",
        "[[[]]]",
        "-0.0",
        "\"नमस्ते 😀\"",
    ] {
        assert_eq!(write(&ok(text), 0), text);
    }
}

/// A number keeps its digits (decision 0071): kept as an `i64` or `f64`
/// only when that writes back the same, otherwise by its text; nothing is
/// refused or rounded.
#[test]
fn numbers_keep_their_digits() {
    assert_eq!(ok("42"), Tree::Int(42));
    assert_eq!(ok("-9223372036854775808"), Tree::Int(i64::MIN));
    assert_eq!(ok("42.0"), Tree::Float(42.0));
    assert_eq!(ok("0.1"), Tree::Float(0.1));
    for kept in [
        "4e2",
        "-0",
        "9223372036854775808",
        "1e400",
        "-1e400",
        "0.10",
        "1.2345678901234567890123",
    ] {
        assert_eq!(ok(kept), Tree::Number(kept.to_string()), "{kept}");
        assert_eq!(write(&ok(kept), 0), kept);
    }
    assert!(super::is_number("-2.5e10") && !super::is_number("2.") && !super::is_number("NaN"));
    assert!(super::exponent_fits("1e10000") && !super::exponent_fits("1e10001"));
}

/// `comments: true` reads JSONC: comments and trailing commas, and still
/// nothing else of JSON5.
#[test]
fn comments_and_trailing_commas_only_when_asked() {
    let jsonc = Limits {
        comments: true,
        ..Limits::default()
    };
    let text = "// settings\n{\n  \"a\": [1, 2,], /* two */\n  \"b\": true,\n}";
    assert!(parse(text, &Limits::default()).is_err());
    let tree = parse(text, &jsonc).expect("JSONC");
    assert_eq!(write(&tree, 0), "{\"a\":[1,2],\"b\":true}");
    for json5 in ["NaN", "Infinity", "0x10", "{'a': 1}", "{a: 1}", "[1,,2]"] {
        assert!(parse(json5, &jsonc).is_err(), "{json5}");
    }
}

#[test]
fn bad_text_says_where() {
    for (text, place) in [
        ("[1,]", (1, 4)),
        ("{\"a\" 1}", (1, 6)),
        ("[01]", (1, 2)),
        ("\n  [1 2]", (2, 6)),
        ("\"a\u{1}b\"", (1, 3)),
        ("\"\\x\"", (1, 3)),
        ("\"\\ud800\"", (1, 2)),
        ("[1] x", (1, 5)),
        ("", (1, 1)),
        ("tru", (1, 1)),
    ] {
        assert_eq!(syntax_at(&err(text)), place, "{text:?}");
    }
}

#[test]
fn surrogate_pairs_make_one_character() {
    assert_eq!(ok("\"\\ud83d\\ude00\""), Tree::Text("😀".into()));
}

#[test]
fn limits_stop_hostile_documents() {
    let deep = "[".repeat(200) + &"]".repeat(200);
    assert_eq!(err(&deep), Error::TooDeep(128));
    let raised = Limits {
        max_depth: 1_000_000,
        ..Limits::default()
    };
    let very_deep = "[".repeat(1_000) + &"]".repeat(1_000);
    assert!(parse(&very_deep, &raised).is_ok());
    let too_deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
    assert_eq!(parse(&too_deep, &raised), Err(Error::TooDeep(MAX_DEPTH)));
    let small = Limits {
        max_bytes: 4,
        ..Limits::default()
    };
    assert_eq!(parse("[1,2]", &small), Err(Error::TooLarge(4)));
    let few = Limits {
        max_fields: 2,
        ..Limits::default()
    };
    assert_eq!(
        parse("{\"a\":1,\"b\":2,\"c\":3}", &few),
        Err(Error::TooLarge(2))
    );
}

#[test]
fn a_key_given_twice_is_an_error() {
    let e = err("{\"a\":1,\n \"a\":2}");
    assert_eq!(
        e,
        Error::DuplicateKey {
            key: "a".into(),
            line: 2,
            column: 2
        }
    );
    // Large objects check with a set.
    let fields: Vec<String> = (0..40).map(|i| format!("\"k{i}\":{i}")).collect();
    let text = format!("{{{},\"k7\":0}}", fields.join(","));
    assert!(matches!(err(&text), Error::DuplicateKey { .. }));
}

#[test]
fn indented_text_puts_each_item_on_a_line() {
    let tree = ok("{\"a\":[1,2],\"b\":{}}");
    assert_eq!(
        write(&tree, 2),
        "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {}\n}"
    );
    assert_eq!(write(&Tree::Float(f64::NAN), 0), "null");
}

#[test]
fn keys_of_nested_objects_are_kept_apart() {
    assert!(
        parse(
            "{\"a\": {\"a\": 1, \"b\": {\"a\": 2}}, \"b\": [{\"a\": 3}]}",
            &Limits::default()
        )
        .is_ok()
    );
    let e = err("{\"a\": 1, \"b\": {\"a\": 1}, \"a\": 2}");
    assert!(matches!(e, Error::DuplicateKey { key, .. } if key == "a"));
}

/// Reads one value piece by piece, as a decoder does; object values are
/// skipped when `skip` (through `Pull::skip`).
fn pull_walk(r: &mut Pull<'_>, skip: bool) -> Result<(), Error> {
    match r.peek() {
        Some(b'[') => {
            r.array()?;
            let mut first = true;
            while r.item(first)? {
                first = false;
                pull_walk(r, skip)?;
            }
            Ok(())
        }
        Some(b'{') => {
            r.object()?;
            let mut keys = Keys::default();
            while r.field(&mut keys)?.is_some() {
                if skip { r.skip()? } else { pull_walk(r, skip)? }
            }
            Ok(())
        }
        Some(b'"') => r.text().map(|_| ()),
        Some(b't' | b'f') => r.bool().map(|_| ()),
        Some(b'n') => r.null(),
        _ => r.number().map(|_| ()),
    }
}

#[test]
fn reading_piece_by_piece_accepts_exactly_what_parse_accepts() {
    let limits = Limits {
        max_depth: 3,
        max_bytes: 60,
        max_fields: 2,
        comments: false,
    };
    for text in [
        "[[1],[2,[3]]]",
        "[[[[1]]]]",
        "[[]] ",
        "[1,]",
        "[1 2]",
        "{\"a\":{\"b\":[]},\"c\":true}",
        "{\"a\":1,\"a\":2}",
        "{\"a\":1,\"b\":2,\"c\":3}",
        "{\"a\":{\"x\":1,\"y\":2,\"z\":3}}",
        "{\"a\" 1}",
        "{\"a\":1}}",
        "{}",
        "[{}, {\"k\": null}]",
        "\"unclosed",
        r#""bad \q escape""#,
        "nul",
        "01",
        "-",
        "1.5e3",
        "99999999999999999999",
        "[true, false] x",
        "  [ 1 , [ 2 ] ]  ",
        "[\"0123456789012345678901234567890123456789012345678901234567\"]",
    ] {
        let parsed = parse(text, &limits).is_ok();
        for skip in [false, true] {
            let pulled = Pull::new(text, &limits)
                .and_then(|mut r| pull_walk(&mut r, skip).and_then(|_| r.end()))
                .is_ok();
            assert_eq!(pulled, parsed, "{text} (skip {skip})");
        }
    }
}

/// Hostile text never panics the parser, and depth is a limit, not a
/// stack overflow (to-do item 4). Deterministic xorshift, so a failure
/// reproduces; `KAVACH_FUZZ_CASES` raises the count for a long run.
#[test]
fn hostile_text_never_panics_and_depth_is_limited() {
    let cases = std::env::var("KAVACH_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300usize);
    let mut seed = 0x0005_EED4_u64;
    let mut next = move || {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    const PIECES: &[&str] = &[
        "{",
        "}",
        "[",
        "]",
        "\"",
        "\\",
        ":",
        ",",
        " ",
        "\n",
        "true",
        "false",
        "null",
        "-",
        "0",
        "9",
        ".",
        "e",
        "E",
        "+",
        "\\u",
        "\\u00",
        "\\ud800",
        "a",
        "é",
        "\u{0}",
        "\"a\":",
        "1e999",
        "-0.0000000000000000001",
        "99999999999999999999",
    ];
    let limits = Limits::default();
    for _ in 0..cases {
        let len = (next() % 200) as usize;
        let mut text = String::new();
        while text.len() < len {
            text.push_str(PIECES[(next() % PIECES.len() as u64) as usize]);
        }
        let _ = parse(&text, &limits);
    }
    // A million-deep nesting is an error long before the stack ends.
    let deep = "[".repeat(1_000_000);
    assert!(parse(&deep, &limits).is_err());
    let deep_objects = "{\"a\":".repeat(1_000_000);
    assert!(parse(&deep_objects, &limits).is_err());
}

/// The quick test for a fraction that writes back as it was written never
/// says yes when writing it out would differ (decision 0071).
#[test]
fn the_quick_write_back_test_agrees_with_writing() {
    let mut seed: u64 = 0x5EED;
    let mut next = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let digits = |next: &mut dyn FnMut(u64) -> u64, n: u64| {
        (0..n)
            .map(|_| char::from(b'0' + next(10) as u8))
            .collect::<String>()
    };
    let mut checked = 0;
    for _ in 0..200_000 {
        let whole = match next(3) {
            0 => "0".to_string(),
            _ => {
                let (first, more) = (1 + next(9), next(17));
                format!("{first}{}", digits(&mut next, more))
            }
        };
        let (zeros, more) = (next(7) as usize, 1 + next(16));
        let fraction = format!("{}{}", "0".repeat(zeros), digits(&mut next, more));
        let sign = if next(2) == 0 { "-" } else { "" };
        let text = format!("{sign}{whole}.{fraction}");
        if super::read::writes_back(&text) {
            let f: f64 = text.parse().unwrap();
            assert_eq!(crate::format_float(f), text);
            checked += 1;
        }
    }
    assert!(checked > 10_000, "only {checked} took the quick way");
}
