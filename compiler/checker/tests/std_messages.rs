//! Messages in programs that use the standard library, compiled with it as
//! the driver adds it.

fn first_error(src: &str) -> String {
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    c.messages
        .iter()
        .find(|m| m.severity == kavach_driver::Severity::Error)
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

/// A bare variant name in a `match` on an `Error` names the type among
/// those the program uses: `std.devices` has a `NotFound` too, but a
/// program that only uses `std.fs` is told `FsError.NotFound`.
#[test]
fn a_bare_variant_is_named_from_the_modules_used() {
    let src = "use std.fs\n\nfn load(path: String) -> String ! Error {\n    try fs.read_text(path)\n}\n\nfn main() {\n    const text = load(\"app.toml\") catch e => match e {\n        NotFound(p) => \"no {p}\",\n        _ => \"something else\",\n    }\n    print(text)\n}\n";
    assert_eq!(
        first_error(src),
        "an `Error` can hold any error type: write `FsError.NotFound`"
    );
}
