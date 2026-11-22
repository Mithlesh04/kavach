// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
    let src = "use std.fs\n\nfn load(path: String) -> String ! Error {\n    try fs.read_text(path)\n}\n\nfn main() {\n    const text = load(\"app.toml\") catch e => match e {\n        NotFound(p) => `no {p}`,\n        _ => \"something else\",\n    }\n    print(text)\n}\n";
    assert_eq!(
        first_error(src),
        "an `Error` can hold any error type: write `FsError.NotFound`"
    );
}

/// A list of models with an item that isn't a `Model` gets one error, at
/// the item, not also one saying the list can't be worked out (spec 8.4).
#[test]
fn a_wrong_item_in_a_list_of_models_is_one_error() {
    let src = "type V {\n    ok: Bool\n}\n\nfn careful() -> Model {\n    Model(name: \"claude-opus-5-5\")\n}\n\nfn main() {\n    const v = ask<V>(\"Fine?\", model: [careful(), \"claude-haiku-5-5\"]) catch _e => V(ok: false)\n    print(v.ok)\n}\n";
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let errors: Vec<&str> = (c.messages.iter())
        .filter(|m| m.severity == kavach_driver::Severity::Error)
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expected `Model`, found `String`"),
        "{errors:?}"
    );
}
