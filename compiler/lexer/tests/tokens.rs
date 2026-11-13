//! Table-driven tests: each case is a piece of source and the tokens it must
//! produce. Spec section numbers are in the comments.

mod common;
use common::{check_all, errors};

#[test]
fn names_and_keywords() {
    // 1.5, 1.6
    check_all(&[
        ("const total = count", "const total = count"),
        ("नाम नमस्ते _x x1", "नाम नमस्ते _x x1"),
        ("result fails fix context", "result fails fix context"),
        ("self none true", "self none true"),
    ]);
}

#[test]
fn numbers() {
    // 1.7
    check_all(&[
        (
            "42 1_000_000 0xff 0b1010",
            "int:42 int:1000000 int:255 int:10",
        ),
        (
            "3.14 1e9 2.5e-3 0.1",
            "float:3.14 float:1e9 float:2.5e-3 float:0.1",
        ),
        ("0..10 1..=9", "int:0 .. int:10 int:1 ..= int:9"),
        ("x.len() 5.method()", "x . len ( ) int:5 . method ( )"),
        ("18446744073709551615", "int:18446744073709551615"),
    ]);
}

#[test]
fn bad_numbers() {
    check_all(&[
        // Meant as a name: reported, then read as one (spec 1.5).
        ("12abc", "12abc"),
        // Reported, then read as `0`, so they cause no second error.
        ("0x", "int:0"),
        ("0b102", "int:0"),
        ("0x10000000000000000", "int:0"),
        // Too big for 64 bits: a `BigInt` (spec 2.1).
        ("18446744073709551616", "bigint:18446744073709551616"),
    ]);
    assert_eq!(errors("0b102"), ["1: invalid digit in binary number"]);
    assert_eq!(
        errors("2nd_place"),
        ["1: a name can't start with a digit (spec 1.5): start it with a letter, as `place_2nd`"]
    );
}

#[test]
fn characters() {
    // 1.7
    check_all(&[
        (
            "'a' 'क' '\\n' '\\'' '\\u{1F600}'",
            "char:'a' char:'क' char:'\\n' char:'\\'' char:'😀'",
        ),
        ("''", "ERR"),
        ("'ab'", "ERR"),
    ]);
}

#[test]
fn strings() {
    // 1.7
    check_all(&[
        ("\"hello\"", "\"< text:\"hello\" >\""),
        ("\"\"", "\"< >\""),
        (
            "\"say \\\"hi\\\" \\\\ \\u{0915}\"",
            "\"< text:\"say \\\"hi\\\" \\\\ क\" >\"",
        ),
        (
            "\"Use \\{ and } here\"",
            "\"< text:\"Use { and } here\" >\"",
        ),
        ("r\"C:\\path\\{x}\"", "\"< text:\"C:\\\\path\\\\{x}\" >\""),
    ]);
}

#[test]
fn interpolation() {
    // 1.7
    check_all(&[
        (
            "\"Hi {name}!\"",
            "\"< text:\"Hi \" ${ name }$ text:\"!\" >\"",
        ),
        ("\"{a + b}\"", "\"< ${ a + b }$ >\""),
        (
            "\"{f(\"in {x}\")}\"",
            "\"< ${ f ( \"< text:\"in \" ${ x }$ >\" ) }$ >\"",
        ),
        ("\"{ {a} }\"", "\"< ${ { a } }$ >\""),
    ]);
}

#[test]
fn multi_line_strings() {
    // 1.7: line breaks next to the delimiters are dropped, then the common indentation.
    check_all(&[
        (
            "\"\"\"\n    first\n      second\n    \"\"\"",
            "\"< text:\"first\\n  second\" >\"",
        ),
        ("\"\"\"\n  a\n\n  b\n  \"\"\"", "\"< text:\"a\\n\\nb\" >\""),
        (
            "\"\"\"\n  Hi {name},\n    bye\n  \"\"\"",
            "\"< text:\"Hi \" ${ name }$ text:\",\\n  bye\" >\"",
        ),
        (
            "\"\"\"\n  {x}\n  y\n  \"\"\"",
            "\"< ${ x }$ text:\"\\ny\" >\"",
        ),
        ("\"\"\"\n  last\"\"\"", "\"< text:\"last\" >\""),
        ("\"\"\"\n\"\"\"", "\"< >\""),
        (
            "\"\"\"\n  say \"hi\"\n  \"\"\"",
            "\"< text:\"say \\\"hi\\\"\" >\"",
        ),
    ]);
}

#[test]
fn operators_take_the_longest_match() {
    // 1.8
    check_all(&[
        ("a ..= b .. c", "a ..= b .. c"),
        ("a -> b => c", "a -> b => c"),
        ("a == b != c <= d >= e", "a == b != c <= d >= e"),
        ("a ?? b ?. c", "a ?? b ?. c"),
        (
            "a += 1 -= 2 *= 3 /= 4 %= 5",
            "a += int:1 -= int:2 *= int:3 /= int:4 %= int:5",
        ),
        ("a << b >> c & d | e ^ ~f", "a << b >> c & d | e ^ ~ f"),
        ("x: Int? ! E", "x : Int ? ! E"),
    ]);
}

#[test]
fn comments() {
    // 1.2
    check_all(&[
        ("a // note\nb", "a ⏎ b"),
        ("/// Adds.\nfn add", "doc:\"Adds.\" ⏎ fn add"),
        ("//// not a doc comment\nx", "x"),
        // A byte order mark at a file's start is a space (1.1).
        ("\u{feff}// note\nx", "x"),
    ]);
}

#[test]
fn statements_end_at_line_breaks() {
    // 1.1
    check_all(&[
        (
            "const a = 1\nconst b = 2",
            "const a = int:1 ⏎ const b = int:2",
        ),
        ("a\n\n\nb", "a ⏎ b"),
        ("\n\na", "a"),
        ("const t = 1 +\n  2", "const t = int:1 + int:2"),
        ("f(a,\n  b)", "f ( a , b )"),
        (
            "orders\n  .filter(x)\n  .sum(y)",
            "orders . filter ( x ) . sum ( y )",
        ),
        ("a\n  // comment\n  .b", "a . b"),
        ("a and\nb", "a and b"),
        ("[1,\n 2\n]", "[ int:1 , int:2 ]"),
        ("f(\n  a\n)", "f ( a )"),
    ]);
}

#[test]
fn braces_make_line_breaks_count_again() {
    // 1.1: a lambda body inside a call still has statements.
    check_all(&[
        ("f(x => {\n  a\n  b\n})", "f ( x => { a ⏎ b ⏎ } )"),
        ("if a {\n  b\n}\nc", "if a { b ⏎ } ⏎ c"),
    ]);
}

#[test]
fn types_do_not_continue_lines() {
    // 1.1: `>`, `>>` and `?` close types.
    check_all(&[
        ("const s: Set<String>\nx", "const s : Set < String > ⏎ x"),
        (
            "const m: Map<K, List<V>>\nx",
            "const m : Map < K , List < V >> ⏎ x",
        ),
        ("const a: Int?\nx", "const a : Int ? ⏎ x"),
    ]);
}

#[test]
fn errors_are_reported_and_lexing_continues() {
    check_all(&[
        ("const x = 1;", "const x = int:1 ERR"),
        ("/* old */ x", "x"),
        ("a /* one /* two */ still */ b", "a b"),
        ("a @ b", "a ERR b"),
    ]);
    assert_eq!(
        errors("const x = 1;\n\"open\nx"),
        [
            "1: Kavach has no semicolons: a statement ends at the end of its line",
            "2: text can't continue on the next line: close it with `\"`, or use `\"\"\"` for multi-line text",
        ]
    );
    // An unclosed comment runs to the end: nothing after it is read.
    assert_eq!(
        errors("x\n/* a /* b */\n\"open\nx"),
        ["2: this `/*` comment is never closed with `*/`"]
    );
    assert_eq!(errors("\"bad \\q\""), ["1: unknown escape `\\q`"]);
    assert_eq!(
        errors("\"\\u{110000}\""),
        ["1: U+110000 is not a valid Unicode character"]
    );
    assert_eq!(
        errors("\"{x"),
        ["1: this `{` inside a string is never closed with `}`"]
    );
    assert_eq!(errors("\"\"\"\n  open"), ["2: this text is never closed"]);
}

#[test]
fn brackets_are_tracked() {
    assert_eq!(
        errors("f(a))"),
        ["1: unexpected `)`: nothing here to close"]
    );
    assert_eq!(errors("{ f(a }"), ["1: this bracket is never closed"]);
    assert!(errors("f(a, [b, {c}])").is_empty());
}
